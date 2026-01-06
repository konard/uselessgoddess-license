pub use sea_orm_migration::prelude::*;

mod m20251214_000001_create_users;
mod m20251214_000002_create_licenses;
mod m20251214_000003_create_user_stats;
mod m20251214_000004_create_builds;
mod m20251214_000005_create_claimed_promos;
mod m20251214_000006_create_free_games;
mod m20251218_000007_add_detailed_stats;
mod m20251218_000009_create_free_items;
mod m20260104_000010_add_referral_system;
mod m20260104_000011_create_pending_invoices;
mod m20260105_000012_update_commission_default;
mod m20260106_000013_add_referral_code;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
      Box::new(m20251214_000001_create_users::Migration),
      Box::new(m20251214_000002_create_licenses::Migration),
      Box::new(m20251214_000003_create_user_stats::Migration),
      Box::new(m20251214_000004_create_builds::Migration),
      Box::new(m20251214_000005_create_claimed_promos::Migration),
      Box::new(m20251214_000006_create_free_games::Migration),
      Box::new(m20251218_000007_add_detailed_stats::Migration),
      Box::new(m20251218_000009_create_free_items::Migration),
      Box::new(m20260104_000010_add_referral_system::Migration),
      Box::new(m20260104_000011_create_pending_invoices::Migration),
      Box::new(m20260105_000012_update_commission_default::Migration),
      Box::new(m20260106_000013_add_referral_code::Migration),
    ]
  }
}
