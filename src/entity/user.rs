use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::{license, promo, stats};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub tg_user_id: i64,
  pub reg_date: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(has_many = "license::Entity")]
  Licenses,
  #[sea_orm(has_one = "stats::Entity")]
  UserStats,
  #[sea_orm(has_many = "promo::Entity")]
  ClaimedPromos,
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

impl ActiveModelBehavior for ActiveModel {}
