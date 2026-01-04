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
      commission_rate: Set(25),
      discount_percent: Set(3),
      referral_sales: Set(0),
      referral_earnings: Set(0),
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
  #[allow(dead_code)]
  pub async fn set_referred_by(
    &self,
    tg_user_id: i64,
    referrer_id: i64,
  ) -> Result<()> {
    let user = user::Entity::find_by_id(tg_user_id)
      .one(self.db)
      .await?
      .ok_or(Error::UserNotFound)?;

    if user.referred_by.is_some() {
      return Err(Error::InvalidArgs("User already has a referrer".into()));
    }

    // Cannot refer yourself
    if tg_user_id == referrer_id {
      return Err(Error::InvalidArgs("Cannot refer yourself".into()));
    }

    user::ActiveModel { referred_by: Set(Some(referrer_id)), ..user.into() }
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
}
