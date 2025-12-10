#[allow(irrefutable_let_patterns)]
mod bot;
mod handlers;
mod model;
mod prelude;
mod state;

use std::collections::HashSet;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::routing::post;
use state::App;
use teloxide::types::ChatId;
use tokio::time;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::prelude::*;

#[tokio::main]
async fn main() {
  dotenvy::dotenv().ok();

  tracing_subscriber::registry()
    .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
      "license=debug,tower_http=debug,axum=trace,sqlx=warn".into()
    }))
    .with(tracing_subscriber::fmt::layer())
    .init();

  let admins: HashSet<i64> = env::var("ADMIN_IDS")
    .expect("ADMIN_IDS not set")
    .split(',')
    .map(|id| id.trim().parse().expect("Invalid Admin ID format"))
    .collect();

  let db_url = env::var("DATABASE_URL").unwrap_or("sqlite:licenses.db".into());
  let token = env::var("TELOXIDE_TOKEN").expect("No token");
  let secret = env::var("SERVER_SECRET").expect("No secret");

  info!("Starting License Service...");

  let app_state = Arc::new(App::new(&db_url, &token, admins, secret).await);

  let bot_state = app_state.clone();
  tokio::spawn(async move {
    bot::run_bot(bot_state).await;
  });

  let backup_app = app_state.clone();
  if !backup_app.admins.is_empty() {
    tokio::spawn(async move {
      let mut interval = time::interval(Duration::from_hours(1));
      loop {
        interval.tick().await;
        if let Err(err) = backup_app.perform_smart_backup().await {
          error!("Auto-Backup failed: {err}")
        }
      }
    });
  } else {
    eprintln!("Warning: No admins found, auto-backups disabled.");
  }

  let gc_app = app_state.clone();
  tokio::spawn(async move {
    let mut interval = time::interval(Duration::from_secs(60));
    loop {
      interval.tick().await;
      let now = Utc::now().naive_utc();
      gc_app.sessions.retain(|_key, sessions| {
        sessions.retain(|s| (now - s.last_seen).num_seconds() < 120);
        !sessions.is_empty()
      });
    }
  });

  // TODO: add layer logging requests
  let app = Router::new()
    .route("/api/heartbeat", post(handlers::heartbeat))
    .with_state(app_state);

  let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
  info!("Server listening on {}", addr);

  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  axum::serve(listener, app).await.unwrap();
}
