use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::{license, promo, referral_code, stats, transaction};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[derive(EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum UserRole {
  #[sea_orm(string_value = "user")]
  #[default]
  User,
  #[sea_orm(string_value = "creator")]
  Creator,
  #[sea_orm(string_value = "admin")]
  Admin,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub tg_user_id: i64,
  pub reg_date: DateTime,
  pub balance: i64,
  pub role: UserRole,
  pub referred_by: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(has_many = "license::Entity")]
  Licenses,
  #[sea_orm(has_one = "stats::Entity")]
  UserStats,
  #[sea_orm(has_many = "promo::Entity")]
  ClaimedPromos,
  #[sea_orm(has_many = "referral_code::Entity")]
  ReferralCodes,
  #[sea_orm(has_many = "transaction::Entity")]
  Transactions,
}

impl Related<license::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::Licenses.def()
  }
}

impl Related<stats::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::UserStats.def()
  }
}

impl Related<promo::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::ClaimedPromos.def()
  }
}

impl Related<referral_code::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::ReferralCodes.def()
  }
}

impl Related<transaction::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::Transactions.def()
  }
}

impl ActiveModelBehavior for ActiveModel {}
