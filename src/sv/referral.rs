use crate::{
  entity::{referral_code, user::UserRole},
  prelude::*,
  sv,
};

pub struct Referral<'a> {
  db: &'a DatabaseConnection,
}

pub const CENTS_PER_DOLLAR: i64 = 100;
#[allow(dead_code)]
pub const MONTH_PRICE_CENTS: i64 = 10 * CENTS_PER_DOLLAR;
#[allow(dead_code)]
pub const QUARTER_PRICE_CENTS: i64 = 25 * CENTS_PER_DOLLAR;

#[allow(dead_code)]
impl<'a> Referral<'a> {
  pub fn new(db: &'a DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn create_code(
    &self,
    owner_id: i64,
    code: String,
    commission_rate: i32,
    discount_percent: i32,
  ) -> Result<referral_code::Model> {
    let user = sv::User::new(self.db).get_or_create(owner_id).await?;

    if user.role != UserRole::Creator && user.role != UserRole::Admin {
      return Err(Error::InvalidArgs(
        "Only creators and admins can create referral codes".into(),
      ));
    }

    if referral_code::Entity::find_by_id(&code).one(self.db).await?.is_some() {
      return Err(Error::InvalidArgs("Referral code already exists".into()));
    }

    let now = Utc::now().naive_utc();
    let referral = referral_code::ActiveModel {
      code: Set(code),
      owner_id: Set(owner_id),
      commission_rate: Set(commission_rate),
      discount_percent: Set(discount_percent),
      total_sales: Set(0),
      total_earnings: Set(0),
      is_active: Set(true),
      created_at: Set(now),
    };

    Ok(referral.insert(self.db).await?)
  }

  pub async fn by_code(
    &self,
    code: &str,
  ) -> Result<Option<referral_code::Model>> {
    Ok(referral_code::Entity::find_by_id(code).one(self.db).await?)
  }

  pub async fn by_owner(
    &self,
    owner_id: i64,
  ) -> Result<Vec<referral_code::Model>> {
    Ok(
      referral_code::Entity::find()
        .filter(referral_code::Column::OwnerId.eq(owner_id))
        .all(self.db)
        .await?,
    )
  }

  pub async fn validate_code(
    &self,
    code: &str,
  ) -> Result<referral_code::Model> {
    let referral = self.by_code(code).await?.ok_or(Error::ReferralNotFound)?;

    if !referral.is_active {
      return Err(Error::ReferralInactive);
    }

    Ok(referral)
  }

  pub async fn record_sale(
    &self,
    code: &str,
    sale_amount_cents: i64,
  ) -> Result<i64> {
    let txn = self.db.begin().await?;

    let referral = referral_code::Entity::find_by_id(code)
      .one(&txn)
      .await?
      .ok_or(Error::ReferralNotFound)?;

    if !referral.is_active {
      return Err(Error::ReferralInactive);
    }

    let commission =
      (sale_amount_cents * referral.commission_rate as i64) / 100;

    referral_code::ActiveModel {
      total_sales: Set(referral.total_sales + 1),
      total_earnings: Set(referral.total_earnings + commission),
      ..referral.into()
    }
    .update(&txn)
    .await?;

    txn.commit().await?;
    Ok(commission)
  }

  pub async fn set_active(&self, code: &str, active: bool) -> Result<()> {
    let referral = referral_code::Entity::find_by_id(code)
      .one(self.db)
      .await?
      .ok_or(Error::ReferralNotFound)?;

    referral_code::ActiveModel { is_active: Set(active), ..referral.into() }
      .update(self.db)
      .await?;

    Ok(())
  }

  pub async fn update_commission(
    &self,
    code: &str,
    commission_rate: i32,
  ) -> Result<()> {
    let referral = referral_code::Entity::find_by_id(code)
      .one(self.db)
      .await?
      .ok_or(Error::ReferralNotFound)?;

    referral_code::ActiveModel {
      commission_rate: Set(commission_rate),
      ..referral.into()
    }
    .update(self.db)
    .await?;

    Ok(())
  }

  pub async fn all(&self) -> Result<Vec<referral_code::Model>> {
    Ok(referral_code::Entity::find().all(self.db).await?)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{entity::*, sv::test_utils::test_db};

  #[tokio::test]
  async fn test_create_referral_code() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(0),
      role: Set(UserRole::Creator),
      referred_by: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let code = Referral::new(&db)
      .create_code(12345, "TEST123".to_string(), 25, 3)
      .await
      .unwrap();

    assert_eq!(code.code, "TEST123");
    assert_eq!(code.commission_rate, 25);
    assert_eq!(code.discount_percent, 3);
    assert!(code.is_active);
  }

  #[tokio::test]
  async fn test_regular_user_cannot_create_code() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
      tg_user_id: Set(12345),
      reg_date: Set(now),
      balance: Set(0),
      role: Set(UserRole::User),
      referred_by: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let result =
      Referral::new(&db).create_code(12345, "TEST123".to_string(), 25, 3).await;

    assert!(result.is_err());
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
    }
    .insert(&db)
    .await
    .unwrap();

    Referral::new(&db)
      .create_code(12345, "TEST123".to_string(), 25, 3)
      .await
      .unwrap();

    let commission = Referral::new(&db)
      .record_sale("TEST123", MONTH_PRICE_CENTS)
      .await
      .unwrap();

    assert_eq!(commission, 250); // 25% of 1000 cents

    let code = Referral::new(&db).by_code("TEST123").await.unwrap().unwrap();
    assert_eq!(code.total_sales, 1);
    assert_eq!(code.total_earnings, 250);
  }
}
