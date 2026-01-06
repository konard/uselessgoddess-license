use crate::{
  entity::{TransactionType, transaction, user, user::UserRole},
  prelude::*,
};

pub struct Balance<'a> {
  db: &'a DatabaseConnection,
}

#[allow(dead_code)]
impl<'a> Balance<'a> {
  pub fn new(db: &'a DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn get(&self, user_id: i64) -> Result<i64> {
    let user = user::Entity::find_by_id(user_id)
      .one(self.db)
      .await?
      .ok_or(Error::UserNotFound)?;
    Ok(user.balance)
  }

  pub async fn deposit(
    &self,
    user_id: i64,
    amount: i64,
    description: Option<String>,
  ) -> Result<i64> {
    if amount <= 0 {
      return Err(Error::InvalidArgs("Deposit amount must be positive".into()));
    }

    let txn = self.db.begin().await?;

    let user = user::Entity::find_by_id(user_id)
      .one(&txn)
      .await?
      .ok_or(Error::UserNotFound)?;

    // TODO: use atomic update
    let new_balance = user.balance + amount;

    user::ActiveModel { balance: Set(new_balance), ..user.into() }
      .update(&txn)
      .await?;

    let now = Utc::now().naive_utc();
    transaction::ActiveModel {
      id: NotSet,
      user_id: Set(user_id),
      amount: Set(amount),
      tx_type: Set(TransactionType::Deposit),
      description: Set(description),
      referrer_id: Set(None),
      created_at: Set(now),
    }
    .insert(&txn)
    .await?;

    txn.commit().await?;
    Ok(new_balance)
  }

  pub async fn spend(
    &self,
    user_id: i64,
    amount: i64,
    description: Option<String>,
    referrer_id: Option<i64>,
  ) -> Result<i64> {
    if amount <= 0 {
      return Err(Error::InvalidArgs("Spend amount must be positive".into()));
    }

    let txn = self.db.begin().await?;

    let user = user::Entity::find_by_id(user_id)
      .one(&txn)
      .await?
      .ok_or(Error::UserNotFound)?;

    if user.balance < amount {
      return Err(Error::InsufficientBalance);
    }

    let new_balance = user.balance - amount;

    user::ActiveModel { balance: Set(new_balance), ..user.into() }
      .update(&txn)
      .await?;

    let now = Utc::now().naive_utc();
    transaction::ActiveModel {
      id: NotSet,
      user_id: Set(user_id),
      amount: Set(-amount),
      tx_type: Set(TransactionType::Purchase),
      description: Set(description),
      referrer_id: Set(referrer_id),
      created_at: Set(now),
    }
    .insert(&txn)
    .await?;

    txn.commit().await?;
    Ok(new_balance)
  }

  pub async fn add_referral_bonus(
    &self,
    user_id: i64,
    amount: i64,
    referrer_id: i64,
  ) -> Result<i64> {
    if amount <= 0 {
      return Err(Error::InvalidArgs("Bonus amount must be positive".into()));
    }

    let txn = self.db.begin().await?;

    let user = user::Entity::find_by_id(user_id)
      .one(&txn)
      .await?
      .ok_or(Error::UserNotFound)?;

    // TODO: use atomic update
    let new_balance = user.balance + amount;

    user::ActiveModel { balance: Set(new_balance), ..user.into() }
      .update(&txn)
      .await?;

    let now = Utc::now().naive_utc();
    transaction::ActiveModel {
      id: NotSet,
      user_id: Set(user_id),
      amount: Set(amount),
      tx_type: Set(TransactionType::ReferralBonus),
      description: Set(Some(format!(
        "Referral bonus from user {}",
        referrer_id
      ))),
      referrer_id: Set(Some(referrer_id)),
      created_at: Set(now),
    }
    .insert(&txn)
    .await?;

    txn.commit().await?;
    Ok(new_balance)
  }

  pub async fn withdraw(&self, user_id: i64, amount: i64) -> Result<i64> {
    if amount <= 0 {
      return Err(Error::InvalidArgs(
        "Withdrawal amount must be positive".into(),
      ));
    }

    let txn = self.db.begin().await?;

    let user = user::Entity::find_by_id(user_id)
      .one(&txn)
      .await?
      .ok_or(Error::UserNotFound)?;

    if user.role != UserRole::Creator && user.role != UserRole::Admin {
      return Err(Error::WithdrawalNotAllowed);
    }

    if user.balance < amount {
      return Err(Error::InsufficientBalance);
    }

    let new_balance = user.balance - amount;

    user::ActiveModel { balance: Set(new_balance), ..user.into() }
      .update(&txn)
      .await?;

    let now = Utc::now().naive_utc();
    transaction::ActiveModel {
      id: NotSet,
      user_id: Set(user_id),
      amount: Set(-amount),
      tx_type: Set(TransactionType::Withdrawal),
      description: Set(Some("Crypto withdrawal".to_string())),
      referrer_id: Set(None),
      created_at: Set(now),
    }
    .insert(&txn)
    .await?;

    txn.commit().await?;
    Ok(new_balance)
  }

  pub async fn transactions(
    &self,
    user_id: i64,
    limit: u64,
  ) -> Result<Vec<transaction::Model>> {
    Ok(
      transaction::Entity::find()
        .filter(transaction::Column::UserId.eq(user_id))
        .order_by_desc(transaction::Column::CreatedAt)
        .limit(limit)
        .all(self.db)
        .await?,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{entity::*, sv::test_utils::test_db};

  #[tokio::test]
  async fn test_deposit() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(0),
      role: Set(UserRole::User),
      referred_by: Set(None),
      commission_rate: Set(25),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
      referral_code: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let new_balance = Balance::new(&db)
      .deposit(12345, 1000, Some("Test deposit".into()))
      .await
      .unwrap();

    assert_eq!(new_balance, 1000);
  }

  #[tokio::test]
  async fn test_spend() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(1000),
      role: Set(UserRole::User),
      referred_by: Set(None),
      commission_rate: Set(25),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
      referral_code: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let new_balance = Balance::new(&db)
      .spend(12345, 500, Some("License purchase".into()), None)
      .await
      .unwrap();

    assert_eq!(new_balance, 500);
  }

  #[tokio::test]
  async fn test_insufficient_balance() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(100),
      role: Set(UserRole::User),
      referred_by: Set(None),
      commission_rate: Set(25),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
      referral_code: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let result = Balance::new(&db).spend(12345, 500, None, None).await;

    assert!(matches!(result, Err(Error::InsufficientBalance)));
  }

  #[tokio::test]
  async fn test_withdrawal_requires_creator_role() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(1000),
      role: Set(UserRole::User),
      referred_by: Set(None),
      commission_rate: Set(25),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
      referral_code: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let result = Balance::new(&db).withdraw(12345, 500).await;

    assert!(matches!(result, Err(Error::WithdrawalNotAllowed)));
  }

  #[tokio::test]
  async fn test_creator_can_withdraw() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(1000),
      role: Set(UserRole::Creator),
      referred_by: Set(None),
      commission_rate: Set(25),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
      referral_code: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let new_balance = Balance::new(&db).withdraw(12345, 500).await.unwrap();

    assert_eq!(new_balance, 500);
  }
}
