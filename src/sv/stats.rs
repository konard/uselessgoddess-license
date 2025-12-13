use std::io::Read;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use crate::{entity::*, prelude::*, sv};

/// System stats collected from client for debug analyzing.
/// These structs are used for deserializing client telemetry data.
/// Fields are populated during JSON parsing but may not all be directly read.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SystemStats {
  pub app_version: String,
  pub session_id: String,
  pub hwid_hash: String,
  pub uptime: u64,
  pub performance: PerformanceStats,
  pub farming: FarmingStats,
  pub network: NetworkStats,
  #[serde(default)]
  pub errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PerformanceStats {
  pub avg_fps: f32,
  pub avg_ram_mb: u32,
  pub avg_ai_ms: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FarmingStats {
  pub cycle_time: u32,
  #[serde(default)]
  pub states_stuck: HashMap<String, u32>,
  #[serde(default)]
  pub xp_gained: u64,
  #[serde(default)]
  pub drops: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct NetworkStats {
  #[serde(default)]
  pub srt: HashMap<String, ServerRegionStats>,
  pub avg_ping: u32,
  #[serde(default)]
  pub gc_timeouts: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ServerRegionStats {
  pub ping: u32,
}

#[derive(Debug, Serialize)]
pub struct UserStatsDisplay {
  pub weekly_xp: u64,
  pub total_xp: u64,
  pub drops_count: u32,
  pub instances: u32,
  pub runtime_hours: f64,
}

pub struct Stats<'a> {
  db: &'a DatabaseConnection,
}

impl<'a> Stats<'a> {
  pub fn new(db: &'a DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn get_or_create(&self, tg_user_id: i64) -> Result<stats::Model> {
    if let Some(stats) =
      stats::Entity::find_by_id(tg_user_id).one(self.db).await?
    {
      return Ok(stats);
    }

    sv::User::new(self.db).get_or_create(tg_user_id).await?;

    let now = Utc::now().naive_utc();
    let stats = stats::ActiveModel {
      tg_user_id: Set(tg_user_id),
      weekly_xp: Set(0),
      total_xp: Set(0),
      drops_count: Set(0),
      instances: Set(0),
      runtime_hours: Set(0.0),
      last_updated: Set(now),
    };

    Ok(stats.insert(self.db).await?)
  }

  pub async fn update_from_telemetry(
    &self,
    tg_user_id: i64,
    stats: &SystemStats,
    instances: u32,
  ) -> Result<()> {
    let model = self.get_or_create(tg_user_id).await?;
    let now = Utc::now().naive_utc();

    stats::ActiveModel {
      weekly_xp: Set(model.weekly_xp + stats.farming.xp_gained as i64),
      total_xp: Set(model.total_xp + stats.farming.xp_gained as i64),
      drops_count: Set(model.drops_count + stats.farming.drops as i32),
      runtime_hours: Set(model.runtime_hours + stats.uptime as f64 / 3600.0),
      instances: Set(instances as i32),
      last_updated: Set(now),
      ..model.into()
    }
    .update(self.db)
    .await?;

    Ok(())
  }

  pub async fn display_stats(
    &self,
    tg_user_id: i64,
  ) -> Result<UserStatsDisplay> {
    let stats = self.get_or_create(tg_user_id).await?;

    Ok(UserStatsDisplay {
      weekly_xp: stats.weekly_xp as u64,
      total_xp: stats.total_xp as u64,
      drops_count: stats.drops_count as u32,
      instances: stats.instances as u32,
      runtime_hours: stats.runtime_hours,
    })
  }

  pub fn decompress_stats(compressed: &[u8]) -> Result<SystemStats> {
    let mut decoder = GzDecoder::new(compressed);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).map_err(|e| {
      Error::Internal(format!("Failed to decompress stats: {}", e))
    })?;

    json::from_str(&decompressed).map_err(|e| {
      Error::Internal(format!("Failed to parse stats JSON: {}", e))
    })
  }

  pub async fn reset_weekly_xp(db: &DatabaseConnection) -> Result<()> {
    use sea_orm::sea_query::Expr;

    stats::Entity::update_many()
      .col_expr(stats::Column::WeeklyXp, Expr::value(0i64))
      .exec(db)
      .await?;

    Ok(())
  }

  #[allow(dead_code)]
  pub async fn aggregate(&self) -> Result<AggregatedStats> {
    use sea_orm::sea_query::Expr;

    type StatsRow = (Option<i64>, Option<i64>, Option<i64>, Option<f64>);
    let result: Option<StatsRow> = stats::Entity::find()
      .select_only()
      .column_as(Expr::col(stats::Column::TotalXp).sum(), "total_xp")
      .column_as(Expr::col(stats::Column::WeeklyXp).sum(), "weekly_xp")
      .column_as(Expr::col(stats::Column::DropsCount).sum(), "drops")
      .column_as(Expr::col(stats::Column::RuntimeHours).sum(), "runtime")
      .into_tuple()
      .one(self.db)
      .await?;

    let active_instances: Option<i64> = stats::Entity::find()
      .select_only()
      .column_as(Expr::col(stats::Column::Instances).sum(), "instances")
      .into_tuple()
      .one(self.db)
      .await?;

    Ok(AggregatedStats {
      total_xp: result.and_then(|r| r.0).unwrap_or(0) as u64,
      weekly_xp: result.and_then(|r| r.1).unwrap_or(0) as u64,
      total_drops: result.and_then(|r| r.2).unwrap_or(0) as u64,
      total_runtime_hours: result.and_then(|r| r.3).unwrap_or(0.0),
      active_instances: active_instances.unwrap_or(0) as u32,
    })
  }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AggregatedStats {
  pub total_xp: u64,
  pub weekly_xp: u64,
  pub total_drops: u64,
  pub total_runtime_hours: f64,
  pub active_instances: u32,
}
