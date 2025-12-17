use std::sync::Arc;

use async_trait::async_trait;
use tracing::{error, info};

use crate::{plugins::Plugin, prelude::*, state::AppState, sv};

pub struct GC;

#[async_trait]
impl Plugin for GC {
  async fn start(&self, app: Arc<AppState>) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
      interval.tick().await;
      app.gc_sessions();
      app.gc_download_tokens();
    }
  }
}

pub struct Backup;

#[async_trait]
impl Plugin for Backup {
  async fn start(&self, app: Arc<AppState>) -> anyhow::Result<()> {
    if app.admins.is_empty() {
      warn!("No admins configured, auto-backups disabled");
      return Ok(());
    }

    let interval_hours = app.config.backup_hours;
    if interval_hours == 0 {
      info!("Auto-backups disabled via config (0 hours)");
      return Ok(());
    }

    info!("Backup service started (Interval: {}h)", interval_hours);

    let mut interval = time::interval(Duration::from_hours(interval_hours));

    // skip at the moment backup
    interval.tick().await;

    loop {
      interval.tick().await;

      info!("Starting scheduled backup...");
      if let Err(err) = app.perform_smart_backup().await {
        error!("Auto-backup failed: {}", err);
      }
    }
  }
}

pub struct StatsClean;

#[async_trait]
impl Plugin for StatsClean {
  async fn start(&self, app: Arc<AppState>) -> anyhow::Result<()> {
    loop {
      let now = Utc::now();

      // Calculate days until next Monday
      // num_days_from_monday() returns 0 for Monday, 1 for Tuesday, etc.
      let days_from_monday = now.weekday().num_days_from_monday();
      let days_until_next_monday = if days_from_monday == 0 {
        7 // It's Monday, schedule for next Monday
      } else {
        7 - days_from_monday
      };

      // TODO: check this out
      let next_monday = now
        .date_naive()
        .checked_add_days(chrono::Days::new(days_until_next_monday as u64))
        .expect("Date overflow")
        .and_hms_opt(0, 0, 0)
        .expect("Invalid time");

      let sleep_duration = (next_monday - now.naive_utc())
        .to_std()
        .unwrap_or(Duration::from_secs(3600));

      info!(
        "Weekly stats reset scheduled in {} hours",
        sleep_duration.as_secs() / 3600
      );
      tokio::time::sleep(sleep_duration).await;

      match sv::Stats::reset_weekly_xp(&app.db).await {
        Ok(_) => info!("Weekly XP stats reset successfully"),
        Err(e) => error!("Failed to reset weekly stats: {}", e),
      }
    }
  }
}

pub struct Sync;

#[async_trait]
impl Plugin for Sync {
  async fn start(&self, app: Arc<AppState>) -> anyhow::Result<()> {
    time::sleep(Duration::from_secs(10)).await;
    loop {
      info!("Starting external sync...");
      if let Err(e) = run_sync(&app).await {
        error!("external sync failed: {}", e);
      }
      time::sleep(Duration::from_hours(24)).await;
    }
  }
}

async fn run_sync(_app: &Arc<AppState>) -> anyhow::Result<()> {
  // Здесь используем reqwest или wreq
  // let client = reqwest::Client::new();
  // let data = client.get("https://tls.peet.ws/api/all").send().await?.text().await?;

  // info!("Fetched {} bytes from external API", data.len());
  // app.plugin_cache.insert("tls_data".to_string(), data);

  Ok(())
}
