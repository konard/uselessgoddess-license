use sea_orm_migration::prelude::*;

use super::m20251214_000001_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .add_column(
            ColumnDef::new(UsersExt::Balance)
              .big_integer()
              .not_null()
              .default(0),
          )
          .add_column(
            ColumnDef::new(UsersExt::Role)
              .string()
              .not_null()
              .default("user"),
          )
          .add_column(ColumnDef::new(UsersExt::ReferredBy).string().null())
          .to_owned(),
      )
      .await?;

    manager
      .create_table(
        Table::create()
          .table(ReferralCodes::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(ReferralCodes::Code)
              .string()
              .not_null()
              .primary_key(),
          )
          .col(ColumnDef::new(ReferralCodes::OwnerId).big_integer().not_null())
          .col(
            ColumnDef::new(ReferralCodes::CommissionRate)
              .integer()
              .not_null()
              .default(25),
          )
          .col(
            ColumnDef::new(ReferralCodes::DiscountPercent)
              .integer()
              .not_null()
              .default(3),
          )
          .col(
            ColumnDef::new(ReferralCodes::TotalSales)
              .integer()
              .not_null()
              .default(0),
          )
          .col(
            ColumnDef::new(ReferralCodes::TotalEarnings)
              .big_integer()
              .not_null()
              .default(0),
          )
          .col(
            ColumnDef::new(ReferralCodes::IsActive)
              .boolean()
              .not_null()
              .default(true),
          )
          .col(ColumnDef::new(ReferralCodes::CreatedAt).date_time().not_null())
          .foreign_key(
            ForeignKey::create()
              .name("fk_referral_codes_owner")
              .from(ReferralCodes::Table, ReferralCodes::OwnerId)
              .to(Users::Table, Users::TgUserId)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name("idx_referral_codes_owner")
          .table(ReferralCodes::Table)
          .col(ReferralCodes::OwnerId)
          .to_owned(),
      )
      .await?;

    manager
      .create_table(
        Table::create()
          .table(Transactions::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(Transactions::Id)
              .integer()
              .not_null()
              .auto_increment()
              .primary_key(),
          )
          .col(ColumnDef::new(Transactions::UserId).big_integer().not_null())
          .col(ColumnDef::new(Transactions::Amount).big_integer().not_null())
          .col(ColumnDef::new(Transactions::TxType).string().not_null())
          .col(ColumnDef::new(Transactions::Description).string().null())
          .col(ColumnDef::new(Transactions::ReferralCode).string().null())
          .col(ColumnDef::new(Transactions::CreatedAt).date_time().not_null())
          .foreign_key(
            ForeignKey::create()
              .name("fk_transactions_user")
              .from(Transactions::Table, Transactions::UserId)
              .to(Users::Table, Users::TgUserId)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name("idx_transactions_user")
          .table(Transactions::Table)
          .col(Transactions::UserId)
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(Transactions::Table).to_owned())
      .await?;

    manager
      .drop_table(Table::drop().table(ReferralCodes::Table).to_owned())
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::Balance)
          .drop_column(UsersExt::Role)
          .drop_column(UsersExt::ReferredBy)
          .to_owned(),
      )
      .await
  }
}

#[derive(DeriveIden)]
pub enum UsersExt {
  Balance,
  Role,
  ReferredBy,
}

#[derive(DeriveIden)]
pub enum ReferralCodes {
  Table,
  Code,
  OwnerId,
  CommissionRate,
  DiscountPercent,
  TotalSales,
  TotalEarnings,
  IsActive,
  CreatedAt,
}

#[derive(DeriveIden)]
pub enum Transactions {
  Table,
  Id,
  UserId,
  Amount,
  TxType,
  Description,
  ReferralCode,
  CreatedAt,
}
