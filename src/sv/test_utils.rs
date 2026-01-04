//! Shared test utilities for database setup

#[cfg(test)]
pub mod test_db {
  use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Schema};

  use crate::entity::*;

  /// Creates an in-memory SQLite database with all required tables
  pub async fn setup() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let schema = Schema::new(DbBackend::Sqlite);

    // Create user table
    let stmt = schema.create_table_from_entity(user::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await.unwrap();

    // Create license table
    let stmt = schema.create_table_from_entity(license::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await.unwrap();

    // Create promo table
    let stmt = schema.create_table_from_entity(promo::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await.unwrap();

    // Create referral_code table
    let stmt = schema.create_table_from_entity(referral_code::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await.unwrap();

    // Create transaction table
    let stmt = schema.create_table_from_entity(transaction::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await.unwrap();

    db
  }
}
