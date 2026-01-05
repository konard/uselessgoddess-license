use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::user;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "pending_invoices")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub invoice_id: i64,
  pub user_id: i64,
  pub amount_nano: i64,
  pub referrer_id: Option<i64>,
  pub created_at: DateTime,
  pub expires_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(
    belongs_to = "user::Entity",
    from = "Column::UserId",
    to = "user::Column::TgUserId"
  )]
  User,
}

impl Related<user::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::User.def()
  }
}

impl ActiveModelBehavior for ActiveModel {}
