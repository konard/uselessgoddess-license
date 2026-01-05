use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use teloxide::{prelude::*, types::ParseMode};
use tracing::{debug, error, info, warn};

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

/// Garbage collector for yanked builds that removes oldest yanked builds
/// when disk space falls below configured threshold
pub struct YankedBuildsGC;

#[async_trait]
impl Plugin for YankedBuildsGC {
  async fn start(&self, app: Arc<AppState>) -> anyhow::Result<()> {
    let interval_secs = app.config.gc_check_interval_secs;
    if interval_secs == 0 {
      info!("YankedBuildsGC disabled via config (0 interval)");
      return Ok(());
    }

    info!(
      "YankedBuildsGC started (check interval: {}s, min free space: {}MB)",
      interval_secs,
      app.config.gc_min_free_space / (1024 * 1024)
    );

    let mut interval = time::interval(Duration::from_secs(interval_secs));

    loop {
      interval.tick().await;

      if let Err(e) = run_yanked_builds_gc(&app).await {
        error!("YankedBuildsGC failed: {}", e);
      }
    }
  }
}

/// Get available disk space for the builds directory
fn get_available_space(builds_dir: &str) -> Option<u64> {
  let path = Path::new(builds_dir);

  // Try to get the actual directory or fall back to current dir
  let check_path = if path.exists() {
    path.to_path_buf()
  } else {
    std::env::current_dir().ok()?
  };

  #[cfg(unix)]
  {
    let stat = nix_statvfs(&check_path)?;
    Some(stat.available_space)
  }

  #[cfg(not(unix))]
  {
    // Fallback: assume enough space on non-unix systems
    let _ = check_path;
    Some(u64::MAX)
  }
}

#[cfg(unix)]
struct StatvfsResult {
  available_space: u64,
}

#[cfg(unix)]
fn nix_statvfs(path: &Path) -> Option<StatvfsResult> {
  use std::{ffi::CString, mem::MaybeUninit, os::unix::ffi::OsStrExt};

  let path_cstr = CString::new(path.as_os_str().as_bytes()).ok()?;

  unsafe {
    let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
    if libc::statvfs(path_cstr.as_ptr(), stat.as_mut_ptr()) == 0 {
      let stat = stat.assume_init();
      // Available space = f_bavail * f_frsize
      let available = stat.f_bavail * stat.f_frsize;
      Some(StatvfsResult { available_space: available })
    } else {
      None
    }
  }
}

async fn run_yanked_builds_gc(app: &Arc<AppState>) -> anyhow::Result<()> {
  let min_free_space = app.config.gc_min_free_space;
  let builds_dir = &app.config.builds_directory;

  // Check current free space
  let free_space = match get_available_space(builds_dir) {
    Some(space) => space,
    None => {
      debug!("Could not determine free disk space, skipping GC");
      return Ok(());
    }
  };

  if free_space >= min_free_space {
    debug!(
      "Sufficient disk space: {}MB free (min: {}MB)",
      free_space / (1024 * 1024),
      min_free_space / (1024 * 1024)
    );
    return Ok(());
  }

  info!(
    "Low disk space detected: {}MB free (min: {}MB), starting GC",
    free_space / (1024 * 1024),
    min_free_space / (1024 * 1024)
  );

  let sv = app.sv();
  let yanked_builds = sv.build.yanked_oldest_first().await?;

  if yanked_builds.is_empty() {
    warn!("No yanked builds available to clean up");

    // Notify admins that we can't free up space
    notify_admins_no_yanked_builds(app, free_space, min_free_space).await;
    return Ok(());
  }

  // Delete oldest yanked builds until we have enough space
  let mut deleted_count = 0;
  let mut freed_bytes: u64 = 0;

  for build in yanked_builds {
    // Check file size before deleting
    let file_size =
      std::fs::metadata(&build.file_path).map(|m| m.len()).unwrap_or(0);

    match sv.build.delete(&build.version).await {
      Ok(_) => {
        info!(
          "GC: Deleted yanked build v{} ({}MB, {} downloads)",
          build.version,
          file_size / (1024 * 1024),
          build.downloads
        );
        deleted_count += 1;
        freed_bytes += file_size;

        // Check if we've freed enough space
        if let Some(new_free) = get_available_space(builds_dir)
          && new_free >= min_free_space
        {
          info!(
            "GC complete: freed {}MB, now {}MB free",
            freed_bytes / (1024 * 1024),
            new_free / (1024 * 1024)
          );
          break;
        }
      }
      Err(e) => {
        error!("GC: Failed to delete build v{}: {}", build.version, e);
      }
    }
  }

  if deleted_count > 0 {
    info!(
      "GC: Removed {} yanked build(s), freed ~{}MB",
      deleted_count,
      freed_bytes / (1024 * 1024)
    );
  }

  // Check if we still don't have enough space after deleting all yanked builds
  if let Some(final_free) = get_available_space(builds_dir)
    && final_free < min_free_space
  {
    warn!(
      "GC: Still low on space after cleanup ({}MB free)",
      final_free / (1024 * 1024)
    );

    // Check if there are any more yanked builds
    let remaining = sv.build.yanked_oldest_first().await?;
    if remaining.is_empty() {
      notify_admins_no_yanked_builds(app, final_free, min_free_space).await;
    }
  }

  Ok(())
}

async fn notify_admins_no_yanked_builds(
  app: &AppState,
  current_free: u64,
  min_required: u64,
) {
  if app.admins.is_empty() {
    return;
  }

  let message = format!(
    "⚠️ <b>Disk Space Warning</b>\n\n\
    Low disk space detected but <b>no yanked builds available</b> to clean up.\n\n\
    <b>Current free:</b> {}MB\n\
    <b>Minimum required:</b> {}MB\n\n\
    Please manually free up disk space or yank older builds.",
    current_free / (1024 * 1024),
    min_required / (1024 * 1024)
  );

  for &admin_id in &app.admins {
    let _ = app
      .bot
      .send_message(ChatId(admin_id), &message)
      .parse_mode(ParseMode::Html)
      .await;
  }

  warn!(
    "Notified {} admin(s) about low disk space with no yanked builds",
    app.admins.len()
  );
}
