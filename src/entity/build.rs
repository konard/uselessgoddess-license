use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "builds")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: u64,
  pub version: String,
  pub file_path: String,
  pub changelog: Option<String>,
  pub is_active: bool,
  pub created_at: DateTime,
  pub downloads: u64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
