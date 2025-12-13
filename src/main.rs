#![allow(irrefutable_let_patterns)]

mod bot;
mod entity;
mod error;
mod handlers;
mod migration;
mod prelude;
mod state;
mod sv;

use std::{
  collections::HashSet, env, net::SocketAddr, sync::Arc, time::Duration,
};

use axum::{
  Router,
  routing::{get, post},
};
use tower::ServiceBuilder;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
  cors::{Any, CorsLayer},
  trace::TraceLayer,
};
use tracing_subscriber::{
  EnvFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{prelude::*, state::AppState};

#[tokio::main]
async fn main() {
  dotenvy::dotenv().ok();

  tracing_subscriber::registry()
    .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
      "license=debug,tower_http=debug,axum=trace,sea_orm=warn".into()
    }))
    .with(tracing_subscriber::fmt::layer())
    .init();

  let admins: HashSet<i64> = env::var("ADMIN_IDS")
    .expect("ADMIN_IDS not set")
    .split(',')
    .filter(|s| !s.trim().is_empty())
    .map(|id| id.trim().parse().expect("Invalid Admin ID format"))
    .collect();

  let db_url = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "sqlite:licenses.db?mode=rwc".into());
  let token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN not set");
  let secret = env::var("SERVER_SECRET").expect("SERVER_SECRET not set");

  info!("Starting License Server v{}", env!("CARGO_PKG_VERSION"));

  let app_state =
    Arc::new(AppState::new(&db_url, &token, admins, secret).await);

  let bot_state = app_state.clone();
  tokio::spawn(async move {
    bot::run_bot(bot_state).await;
  });

  let backup_app = app_state.clone();
  if !backup_app.admins.is_empty() {
    tokio::spawn(async move {
      let interval_hours = backup_app.config.backup_hours;
      let mut interval =
        tokio::time::interval(Duration::from_secs(interval_hours * 3600));
      loop {
        interval.tick().await;
        if let Err(err) = backup_app.perform_smart_backup().await {
          error!("Auto-backup failed: {}", err);
        }
      }
    });
  } else {
    warn!("No admins configured, auto-backups disabled");
  }

  // Spawn session garbage collector
  let gc_app = app_state.clone();
  tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
      interval.tick().await;
      gc_app.gc_sessions();
    }
  });

  let governor_conf = Arc::new(
    GovernorConfigBuilder::default()
      .per_second(2)
      .burst_size(100)
      .finish()
      .expect("Failed to build rate limiter config"),
  );

  let governor_limiter = governor_conf.limiter().clone();

  tokio::spawn(async move {
    loop {
      tokio::time::sleep(Duration::from_secs(60)).await;
      governor_limiter.retain_recent();
    }
  });

  let app = Router::new()
    .route("/api/heartbeat", post(handlers::heartbeat))
    .route("/api/stats", post(handlers::submit_stats))
    .route("/health", get(handlers::health))
    .layer(
      ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(GovernorLayer::new(governor_conf))
        .layer(
          CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
        ),
    )
    .with_state(app_state);

  let port: u16 =
    env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
  let addr = SocketAddr::from(([0, 0, 0, 0], port));

  info!("HTTP server listening on {}", addr);

  let listener =
    tokio::net::TcpListener::bind(addr).await.expect("Failed to bind");
  axum::serve(listener, app).await.expect("Server error");
}
