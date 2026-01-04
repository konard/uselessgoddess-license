use uuid::Uuid;

pub use crate::prelude::*;
use crate::{
  entity::{LicenseType, license, promo},
  sv,
};

pub struct License<'a> {
  db: &'a DatabaseConnection,
}

impl<'a> License<'a> {
  pub fn new(db: &'a DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn create(
    &self,
    tg_user_id: i64,
    ty: LicenseType,
    days: u64,
  ) -> Result<license::Model> {
    sv::User::new(self.db).get_or_create(tg_user_id).await?;

    let now = Utc::now().naive_utc();
    let expires_at = now + Duration::from_hours(24 * days);
    let key = Uuid::new_v4();

    let license = license::ActiveModel {
      key: Set(key.to_string()),
      tg_user_id: Set(tg_user_id),
      license_type: Set(ty),
      is_blocked: Set(false),
      expires_at: Set(expires_at),
      created_at: Set(now),
      max_sessions: Set(1), // TODO: based on buy
    };

    Ok(license.insert(self.db).await?)
  }

  pub async fn by_key(&self, key: &str) -> Result<Option<license::Model>> {
    let license = license::Entity::find_by_id(key).one(self.db).await?;
    Ok(license)
  }

  pub async fn by_user(
    &self,
    tg_user_id: i64,
    blocked: bool,
  ) -> Result<Vec<license::Model>> {
    let mut query =
      license::Entity::find().filter(license::Column::TgUserId.eq(tg_user_id));

    if !blocked {
      query = query.filter(license::Column::IsBlocked.eq(false));
    }

    Ok(query.all(self.db).await?)
  }

  pub async fn validate(&self, key: &str) -> Result<license::Model> {
    let license = license::Entity::find_by_id(key)
      .one(self.db)
      .await?
      .ok_or(Error::LicenseNotFound)?;

    let now = Utc::now().naive_utc();
    if license.is_blocked || license.expires_at < now {
      return Err(Error::LicenseInvalid);
    }

    Ok(license)
  }

  pub async fn expires(
    &self,
    key: &str,
    duration: Duration,
  ) -> Result<DateTime> {
    let txn = self.db.begin().await?;

    let license = license::Entity::find_by_id(key)
      .one(&txn)
      .await?
      .ok_or(Error::LicenseNotFound)?;

    let delta = TimeDelta::from_std(duration).unwrap_or(TimeDelta::zero());
    let new_exp = Utc::now().naive_utc() + delta;

    license::ActiveModel {
      expires_at: Set(new_exp),
      is_blocked: Set(false),
      ..license.into()
    }
    .update(&txn)
    .await?;

    txn.commit().await?;
    Ok(new_exp)
  }

  pub async fn set_blocked(&self, key: &str, blocked: bool) -> Result<()> {
    let license = license::Entity::find_by_id(key)
      .one(self.db)
      .await?
      .ok_or(Error::LicenseNotFound)?;

    license::ActiveModel { is_blocked: Set(blocked), ..license.into() }
      .update(self.db)
      .await?;

    Ok(())
  }

  pub fn is_promo_active(&self) -> bool {
    let now = Utc::now();
    // TODO: configurable promo periods
    let start = Utc.with_ymd_and_hms(2025, 12, 14, 13, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2025, 12, 21, 23, 59, 59).unwrap();
    now >= start && now <= end
  }

  #[allow(dead_code)]
  pub async fn count(&self) -> Result<u64> {
    let count = license::Entity::find().count(self.db).await?;
    Ok(count)
  }

  #[allow(dead_code)]
  pub async fn count_active(&self) -> Result<u64> {
    let now = Utc::now().naive_utc();
    let count = license::Entity::find()
      .filter(license::Column::IsBlocked.eq(false))
      .filter(license::Column::ExpiresAt.gt(now))
      .count(self.db)
      .await?;
    Ok(count)
  }

  pub async fn link_to_user(
    &self,
    key: &str,
    tg_user_id: i64,
  ) -> Result<license::Model> {
    // Ensure the user exists
    sv::User::new(self.db).get_or_create(tg_user_id).await?;

    let license = license::Entity::find_by_id(key)
      .one(self.db)
      .await?
      .ok_or(Error::LicenseNotFound)?;

    // Check if license is already linked to a different user
    if license.tg_user_id != 0 && license.tg_user_id != tg_user_id {
      return Err(Error::LicenseAlreadyLinked);
    }

    // Update the license with the new user
    let updated =
      license::ActiveModel { tg_user_id: Set(tg_user_id), ..license.into() }
        .update(self.db)
        .await?;

    Ok(updated)
  }

  pub async fn claim_promo(
    &self,
    tg_user_id: i64,
    promo_name: &str,
  ) -> Result<license::Model> {
    if !self.is_promo_active() {
      return Err(Error::Promo(Promo::Inactive));
    }

    // ensure exists
    sv::User::new(self.db).get_or_create(tg_user_id).await?;

    let existing =
      promo::Entity::find_by_id((tg_user_id, promo_name.to_string()))
        .one(self.db)
        .await?;

    if existing.is_some() {
      return Err(Error::Promo(Promo::Claimed));
    }

    let license = self.create(tg_user_id, LicenseType::Trial, 7).await?;
    let now = Utc::now().naive_utc();

    promo::ActiveModel {
      tg_user_id: Set(tg_user_id),
      promo_name: Set(promo_name.to_string()),
      claimed_at: Set(now),
    }
    .insert(self.db)
    .await?;

    Ok(license)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::sv::test_utils::test_db;

  #[tokio::test]
  async fn test_create_license() {
    let db = test_db::setup().await;

    let license =
      License::new(&db).create(12345, LicenseType::Pro, 30).await.unwrap();

    assert_eq!(license.tg_user_id, 12345);
    assert_eq!(license.license_type, LicenseType::Pro);
    assert!(!license.is_blocked);
  }

  #[tokio::test]
  async fn test_validate_license() {
    let db = test_db::setup().await;
    let sv = License::new(&db);

    let license = sv.create(12345, LicenseType::Trial, 30).await.unwrap();
    let validated = sv.validate(&license.key).await.unwrap();

    assert_eq!(validated.key, license.key);
  }

  #[tokio::test]
  async fn test_block_license() {
    let db = test_db::setup().await;
    let sv = License::new(&db);

    let license = sv.create(12345, LicenseType::Trial, 30).await.unwrap();

    sv.set_blocked(&license.key, true).await.unwrap();

    assert!(matches!(
      sv.validate(&license.key).await,
      Err(Error::LicenseInvalid)
    ));
  }

  #[tokio::test]
  async fn test_extend_license() {
    let db = test_db::setup().await;
    let sv = License::new(&db);

    let license = sv.create(12345, LicenseType::Trial, 1).await.unwrap();

    let old_exp = license.expires_at;
    let new_exp = sv
      .expires(&license.key, Duration::from_secs(30 * 24 * 60 * 60))
      .await
      .unwrap();

    assert!(new_exp > old_exp);
  }
}
