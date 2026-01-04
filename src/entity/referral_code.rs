use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::user;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "referral_codes")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub code: String,
  pub owner_id: i64,
  pub commission_rate: i32,
  pub discount_percent: i32,
  pub total_sales: i32,
  pub total_earnings: i64,
  pub is_active: bool,
  pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(
    belongs_to = "user::Entity",
    from = "Column::OwnerId",
    to = "user::Column::TgUserId"
  )]
  Owner,
}

impl Related<user::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::Owner.def()
  }
}

impl ActiveModelBehavior for ActiveModel {}
