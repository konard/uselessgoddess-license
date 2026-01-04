use sea_orm_migration::prelude::*;

use super::m20251214_000001_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(PendingInvoices::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(PendingInvoices::InvoiceId)
              .big_integer()
              .not_null()
              .primary_key(),
          )
          .col(ColumnDef::new(PendingInvoices::UserId).big_integer().not_null())
          .col(
            ColumnDef::new(PendingInvoices::AmountNano)
              .big_integer()
              .not_null(),
          )
          .col(ColumnDef::new(PendingInvoices::ReferrerId).big_integer().null())
          .col(
            ColumnDef::new(PendingInvoices::CreatedAt)
              .date_time()
              .not_null(),
          )
          .col(
            ColumnDef::new(PendingInvoices::ExpiresAt)
              .date_time()
              .not_null(),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk_pending_invoices_user")
              .from(PendingInvoices::Table, PendingInvoices::UserId)
              .to(Users::Table, Users::TgUserId)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name("idx_pending_invoices_user")
          .table(PendingInvoices::Table)
          .col(PendingInvoices::UserId)
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(PendingInvoices::Table).to_owned())
      .await
  }
}

#[derive(DeriveIden)]
pub enum PendingInvoices {
  Table,
  InvoiceId,
  UserId,
  AmountNano,
  ReferrerId,
  CreatedAt,
  ExpiresAt,
}
