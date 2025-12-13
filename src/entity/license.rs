use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[derive(EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum LicenseType {
  #[sea_orm(string_value = "trial")]
  #[default]
  Trial,
  #[sea_orm(string_value = "pro")]
  Pro,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "licenses")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub key: String,
  pub tg_user_id: i64,
  pub license_type: LicenseType,
  pub expires_at: DateTime,
  pub is_blocked: bool,
  pub created_at: DateTime,
  pub max_sessions: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(
    belongs_to = "super::user::Entity",
    from = "Column::TgUserId",
    to = "super::user::Column::TgUserId"
  )]
  User,
}

impl Related<super::user::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::User.def()
  }
}

impl ActiveModelBehavior for ActiveModel {}
