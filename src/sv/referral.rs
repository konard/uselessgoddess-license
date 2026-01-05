use crate::{
  entity::{user, user::UserRole},
  prelude::*,
};

pub struct Referral<'a> {
  db: &'a DatabaseConnection,
}

/// 1 USDT = 1,000,000 nanoUSDT (USDT uses 6 decimal places)
pub const NANO_USDT: i64 = 1_000_000;
#[allow(dead_code)]
pub const MONTH_PRICE: i64 = 10 * NANO_USDT;
#[allow(dead_code)]
pub const QUARTER_PRICE: i64 = 25 * NANO_USDT;

#[allow(dead_code)]
impl<'a> Referral<'a> {
  pub fn new(db: &'a DatabaseConnection) -> Self {
    Self { db }
  }

  /// Validate a referrer by their user ID
  /// Returns the referrer if they exist (any user can be a referrer)
  /// All users earn commission, but only creators/admins can withdraw
  pub async fn validate_referrer(
    &self,
    referrer_id: i64,
  ) -> Result<user::Model> {
    let referrer = user::Entity::find_by_id(referrer_id)
      .one(self.db)
      .await?
      .ok_or(Error::ReferralNotFound)?;

    Ok(referrer)
  }

  /// Record a sale made through a referrer
  /// Returns the commission amount in nanoUSDT
  /// All users receive commission on their balance
  pub async fn record_sale(
    &self,
    referrer_id: i64,
    sale_amount: i64,
  ) -> Result<i64> {
    let txn = self.db.begin().await?;

    let referrer = user::Entity::find_by_id(referrer_id)
      .one(&txn)
      .await?
      .ok_or(Error::ReferralNotFound)?;

    let commission = (sale_amount * referrer.commission_rate as i64) / 100;

    user::ActiveModel {
      referral_sales: Set(referrer.referral_sales + 1),
      referral_earnings: Set(referrer.referral_earnings + commission),
      balance: Set(referrer.balance + commission),
      ..referrer.into()
    }
    .update(&txn)
    .await?;

    txn.commit().await?;
    Ok(commission)
  }

  /// Get referral stats for a user
  pub async fn stats(&self, user_id: i64) -> Result<ReferralStats> {
    let user = user::Entity::find_by_id(user_id)
      .one(self.db)
      .await?
      .ok_or(Error::UserNotFound)?;

    Ok(ReferralStats {
      commission_rate: user.commission_rate,
      discount_percent: user.discount_percent,
      total_sales: user.referral_sales,
      total_earnings: user.referral_earnings,
      can_withdraw: user.role == UserRole::Creator
        || user.role == UserRole::Admin,
    })
  }

  /// Update commission rate for a user (admin only)
  pub async fn set_commission_rate(
    &self,
    user_id: i64,
    rate: i32,
  ) -> Result<()> {
    let user = user::Entity::find_by_id(user_id)
      .one(self.db)
      .await?
      .ok_or(Error::UserNotFound)?;

    user::ActiveModel { commission_rate: Set(rate), ..user.into() }
      .update(self.db)
      .await?;

    Ok(())
  }

  /// Update discount percent for a user (admin only)
  pub async fn set_discount_percent(
    &self,
    user_id: i64,
    discount: i32,
  ) -> Result<()> {
    let user = user::Entity::find_by_id(user_id)
      .one(self.db)
      .await?
      .ok_or(Error::UserNotFound)?;

    user::ActiveModel { discount_percent: Set(discount), ..user.into() }
      .update(self.db)
      .await?;

    Ok(())
  }

  /// Get all creators (users who can be referrers)
  pub async fn all_creators(&self) -> Result<Vec<user::Model>> {
    Ok(
      user::Entity::find()
        .filter(
          user::Column::Role
            .eq(UserRole::Creator)
            .or(user::Column::Role.eq(UserRole::Admin)),
        )
        .all(self.db)
        .await?,
    )
  }
}

#[derive(Debug)]
pub struct ReferralStats {
  pub commission_rate: i32,
  pub discount_percent: i32,
  pub total_sales: i32,
  pub total_earnings: i64,
  pub can_withdraw: bool,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{entity::*, sv::test_utils::test_db};

  #[tokio::test]
  async fn test_validate_referrer_creator() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(0),
      role: Set(UserRole::Creator),
      referred_by: Set(None),
      commission_rate: Set(25),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
    }
    .insert(&db)
    .await
    .unwrap();

    let referrer = Referral::new(&db).validate_referrer(12345).await.unwrap();
    assert_eq!(referrer.tg_user_id, 12345);
    assert_eq!(referrer.commission_rate, 25);
  }

  #[tokio::test]
  async fn test_regular_user_earns_commission() {
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
    }
    .insert(&db)
    .await
    .unwrap();

    let result = Referral::new(&db).validate_referrer(12345).await;
    assert!(result.is_ok());

    let commission =
      Referral::new(&db).record_sale(12345, MONTH_PRICE).await.unwrap();
    assert_eq!(commission, 2_500_000);

    let user =
      user::Entity::find_by_id(12345i64).one(&db).await.unwrap().unwrap();
    assert_eq!(user.referral_sales, 1);
    assert_eq!(user.referral_earnings, 2_500_000);
    assert_eq!(user.balance, 2_500_000);
  }

  #[tokio::test]
  async fn test_record_sale() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(0),
      role: Set(UserRole::Creator),
      referred_by: Set(None),
      commission_rate: Set(25),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
    }
    .insert(&db)
    .await
    .unwrap();

    let commission =
      Referral::new(&db).record_sale(12345, MONTH_PRICE).await.unwrap();

    // 25% of 10 USDT = 2.5 USDT
    assert_eq!(commission, 2_500_000);

    let user =
      user::Entity::find_by_id(12345i64).one(&db).await.unwrap().unwrap();
    assert_eq!(user.referral_sales, 1);
    assert_eq!(user.referral_earnings, 2_500_000);
    assert_eq!(user.balance, 2_500_000);
  }
}
