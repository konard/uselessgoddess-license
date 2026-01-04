use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::user;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[derive(EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum TransactionType {
  #[sea_orm(string_value = "deposit")]
  #[default]
  Deposit,
  #[sea_orm(string_value = "purchase")]
  Purchase,
  #[sea_orm(string_value = "referral_bonus")]
  ReferralBonus,
  #[sea_orm(string_value = "cashback")]
  Cashback,
  #[sea_orm(string_value = "withdrawal")]
  Withdrawal,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "transactions")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: i32,
  pub user_id: i64,
  pub amount: i64,
  pub tx_type: TransactionType,
  pub description: Option<String>,
  pub referral_code: Option<String>,
  pub created_at: DateTime,
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
