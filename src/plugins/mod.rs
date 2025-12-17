pub mod cron;
pub mod server;
pub mod telegram;

use std::sync::Arc;

use crate::state::AppState;

#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
  fn name(&self) -> &'static str {
    std::any::type_name::<Self>()
  }

  async fn start(&self, app: Arc<AppState>) -> anyhow::Result<()>;
}

pub struct App {
  plugins: Vec<Box<dyn Plugin>>,
}

impl App {
  pub fn new() -> Self {
    Self { plugins: Vec::new() }
  }

  pub fn register<P: Plugin + 'static>(mut self, plugin: P) -> Self {
    self.plugins.push(Box::new(plugin));
    self
  }

  pub async fn run(self, app: Arc<AppState>) {
    for plugin in self.plugins {
      let app = app.clone();
      let name = plugin.name();

      tracing::info!("init `{}`", name);

      if let Err(err) = plugin.start(app).await {
        tracing::error!("failed `{}`: {err}", name);
      }
    }
  }
}
