use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::user;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "claimed_promos")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub tg_user_id: i64,
  #[sea_orm(primary_key, auto_increment = false)]
  pub promo_name: String,
  pub claimed_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(
    belongs_to = "user::Entity",
    from = "Column::TgUserId",
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
