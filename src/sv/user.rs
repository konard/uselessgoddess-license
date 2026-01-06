use crate::{
  entity::{license, user, user::UserRole},
  prelude::*,
};

pub struct User<'a> {
  db: &'a DatabaseConnection,
}

impl<'a> User<'a> {
  pub fn new(db: &'a DatabaseConnection) -> Self {
    Self { db }
  }

  pub async fn get_or_create(&self, tg_user_id: i64) -> Result<user::Model> {
    if let Some(user) =
      user::Entity::find_by_id(tg_user_id).one(self.db).await?
    {
      return Ok(user);
    }

    let now = Utc::now().naive_utc();
    let user = user::ActiveModel {
      tg_user_id: Set(tg_user_id),
      reg_date: Set(now),
      balance: Set(0),
      role: Set(UserRole::User),
      referred_by: Set(None),
      commission_rate: Set(10),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
      referral_code: Set(None),
    };

    Ok(user.insert(self.db).await?)
  }

  pub async fn by_id(&self, tg_user_id: i64) -> Result<Option<user::Model>> {
    let user = user::Entity::find_by_id(tg_user_id).one(self.db).await?;
    Ok(user)
  }

  pub async fn set_role(&self, tg_user_id: i64, role: UserRole) -> Result<()> {
    let user = user::Entity::find_by_id(tg_user_id)
      .one(self.db)
      .await?
      .ok_or(Error::UserNotFound)?;

    user::ActiveModel { role: Set(role), ..user.into() }
      .update(self.db)
      .await?;

    Ok(())
  }

  /// Set the referrer for a user (using referrer's user_id)
  /// Anyone can set any existing user as their referrer
  /// Discount is applied based on the referrer's discount_percent
  /// Commission is only earned if the referrer is a creator/admin
  pub async fn set_referred_by(
    &self,
    tg_user_id: i64,
    referrer_id: Option<i64>,
  ) -> Result<()> {
    let user = user::Entity::find_by_id(tg_user_id)
      .one(self.db)
      .await?
      .ok_or(Error::UserNotFound)?;

    // If setting a new referrer (not clearing)
    if let Some(ref_id) = referrer_id {
      // Cannot refer yourself
      if tg_user_id == ref_id {
        return Err(Error::InvalidArgs("Cannot refer yourself".into()));
      }

      // Validate the referrer exists (any user can be a referrer)
      let _referrer = user::Entity::find_by_id(ref_id)
        .one(self.db)
        .await?
        .ok_or(Error::ReferralNotFound)?;
    }

    user::ActiveModel { referred_by: Set(referrer_id), ..user.into() }
      .update(self.db)
      .await?;

    Ok(())
  }

  #[allow(dead_code)]
  pub async fn all(&self) -> Result<Vec<user::Model>> {
    let users = user::Entity::find()
      .order_by_asc(user::Column::RegDate)
      .all(self.db)
      .await?;
    Ok(users)
  }

  pub async fn all_with_licenses(
    &self,
  ) -> Result<Vec<(user::Model, Vec<license::Model>)>> {
    let users = user::Entity::find()
      .order_by_asc(user::Column::RegDate)
      .find_with_related(license::Entity)
      .all(self.db)
      .await?;
    Ok(users)
  }

  #[allow(dead_code)]
  pub async fn count(&self) -> Result<u64> {
    Ok(user::Entity::find().count(self.db).await?)
  }

  /// Find a user by their custom referral code
  pub async fn by_referral_code(
    &self,
    code: &str,
  ) -> Result<Option<user::Model>> {
    let user = user::Entity::find()
      .filter(user::Column::ReferralCode.eq(code))
      .one(self.db)
      .await?;
    Ok(user)
  }

  /// Set custom referral code for a user (only creators/admins)
  pub async fn set_referral_code(
    &self,
    tg_user_id: i64,
    code: Option<String>,
  ) -> Result<()> {
    let user = user::Entity::find_by_id(tg_user_id)
      .one(self.db)
      .await?
      .ok_or(Error::UserNotFound)?;

    // Only creators and admins can set custom referral codes
    if user.role != UserRole::Creator && user.role != UserRole::Admin {
      return Err(Error::InvalidArgs(
        "Only creators can set custom referral codes".into(),
      ));
    }

    // Validate code format if provided
    if let Some(ref c) = code {
      if c.len() < 3 || c.len() > 20 {
        return Err(Error::InvalidArgs(
          "Referral code must be 3-20 characters".into(),
        ));
      }
      if !c.chars().all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-') {
        return Err(Error::InvalidArgs(
          "Referral code can only contain letters, numbers, underscores, and hyphens".into(),
        ));
      }

      // Prevent codes that are purely numeric to avoid confusion with user IDs
      if c.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(Error::InvalidArgs(
          "Referral code cannot be purely numeric (would conflict with user IDs)".into(),
        ));
      }

      // Check if code is already taken
      if let Some(existing) = self.by_referral_code(c).await?
        && existing.tg_user_id != tg_user_id
      {
        return Err(Error::InvalidArgs("Referral code already taken".into()));
      }
    }

    user::ActiveModel { referral_code: Set(code), ..user.into() }
      .update(self.db)
      .await?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::sv::test_utils::test_db;

  #[tokio::test]
  async fn test_numeric_code_rejected() {
    let db = test_db::setup().await;

    let now = Utc::now().naive_utc();
    // Create a creator user
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
      referral_code: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let user_sv = User::new(&db);

    // Purely numeric code should be rejected
    let result = user_sv.set_referral_code(12345, Some("12345".to_string())).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("purely numeric"));

    // Alphanumeric code should work
    let result = user_sv.set_referral_code(12345, Some("CODE123".to_string())).await;
    assert!(result.is_ok());

    // Code with underscore should work
    let result = user_sv.set_referral_code(12345, Some("my_code".to_string())).await;
    assert!(result.is_ok());
  }
}
