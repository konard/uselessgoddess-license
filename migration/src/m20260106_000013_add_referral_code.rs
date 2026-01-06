use sea_orm_migration::prelude::*;

use super::m20251214_000001_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    // Add referral_code column to users table (nullable, unique)
    // Only creators/admins can set this field
    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .add_column(ColumnDef::new(UsersExt::ReferralCode).string().null())
          .to_owned(),
      )
      .await?;

    // Create unique index for referral_code
    manager
      .create_index(
        Index::create()
          .name("idx_users_referral_code")
          .table(Users::Table)
          .col(UsersExt::ReferralCode)
          .unique()
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_index(
        Index::drop()
          .name("idx_users_referral_code")
          .table(Users::Table)
          .to_owned(),
      )
      .await?;

    manager
      .alter_table(
        Table::alter()
          .table(Users::Table)
          .drop_column(UsersExt::ReferralCode)
          .to_owned(),
      )
      .await
  }
}

#[derive(DeriveIden)]
enum UsersExt {
  ReferralCode,
}
