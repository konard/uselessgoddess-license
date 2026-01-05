use sea_orm_migration::prelude::*;

use super::m20251214_000001_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    // Add balance, role, and referral fields to users table
    // User ID is used directly as referral code
    // SQLite requires separate ALTER TABLE statements for each column
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
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .add_column(
            ColumnDef::new(UsersExt::Role)
              .string()
              .not_null()
              .default("user"),
          )
          .to_owned(),
      )
      .await?;

    // referred_by stores the user_id of the referrer (not a separate code)
    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .add_column(ColumnDef::new(UsersExt::ReferredBy).big_integer().null())
          .to_owned(),
      )
      .await?;

    // Referral settings for this user as a referrer
    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .add_column(
            ColumnDef::new(UsersExt::CommissionRate)
              .integer()
              .not_null()
              .default(25),
          )
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .add_column(
            ColumnDef::new(UsersExt::DiscountPercent)
              .integer()
              .not_null()
              .default(3),
          )
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .add_column(
            ColumnDef::new(UsersExt::ReferralSales)
              .integer()
              .not_null()
              .default(0),
          )
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .add_column(
            ColumnDef::new(UsersExt::ReferralEarnings)
              .big_integer()
              .not_null()
              .default(0),
          )
          .to_owned(),
      )
      .await?;

    // Create transactions table for balance history
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
          // referrer_id stores who referred this purchase (user_id, not a code)
          .col(ColumnDef::new(Transactions::ReferrerId).big_integer().null())
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

    // SQLite requires separate ALTER TABLE statements for each column
    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::ReferralEarnings)
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::ReferralSales)
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::DiscountPercent)
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::CommissionRate)
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::ReferredBy)
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::Role)
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::Balance)
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
  CommissionRate,
  DiscountPercent,
  ReferralSales,
  ReferralEarnings,
}

#[derive(DeriveIden)]
pub enum Transactions {
  Table,
  Id,
  UserId,
  Amount,
  TxType,
  Description,
  ReferrerId,
  CreatedAt,
}
