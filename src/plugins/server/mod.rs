mod handlers;

use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
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

use crate::{prelude::*, state::AppState};

pub struct Plugin;

#[async_trait]
impl super::Plugin for Plugin {
  async fn start(&self, app: Arc<AppState>) -> anyhow::Result<()> {
    let governor_conf = Arc::new(
      GovernorConfigBuilder::default()
        .per_second(2)
        .burst_size(100)
        .finish()
        .context("Failed to build rate limiter config")?,
    );

    let governor_limiter = governor_conf.limiter().clone();

    tokio::spawn(async move {
      loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        governor_limiter.retain_recent();
      }
    });

    let router = Router::new()
      .route("/health", get(handlers::health))
      .route("/api/download", get(handlers::download))
      .route("/api/heartbeat", post(handlers::heartbeat))
      .route("/api/stats", post(handlers::submit_stats))
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
      .with_state(app)
      .into_make_service_with_connect_info::<SocketAddr>();

    let port: u16 =
      std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("HTTP Server listening on {addr}");

    tokio::spawn(async move {
      let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
      axum::serve(listener, router).await.unwrap();
    });

    Ok(())
  }
}
