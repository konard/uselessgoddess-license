use std::{path::Path, sync::Arc, time::Duration};

use reqwest::Url;
use teloxide::{
  prelude::*,
  types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

use super::ReplyBot;
use crate::{
  entity::user::UserRole,
  prelude::*,
  state::{AppState, Services},
  sv::referral::NANO_USDT,
};

/// Callback data enum - provides type-safe callback handling
#[derive(Debug, Clone, PartialEq)]
pub enum Callback {
  Profile,
  License,
  Trial,
  Download,
  DownloadVersion(String),
  Buy,
  BuyPlan(String),
  ExtendLicense,
  ExtendLicenseKey(String),
  ExtendPlan { key: String, plan: String },
  AddFunds,
  PayCryptoAmount(String),
  PayCustomAmount,
  CheckPayments,
  PayManual,
  HaveLicense,
  SetRef,
  Back,
}

impl Callback {
  pub fn to_data(&self) -> String {
    match self {
      Callback::Profile => "profile".to_string(),
      Callback::License => "license".to_string(),
      Callback::Trial => "trial".to_string(),
      Callback::Download => "download".to_string(),
      Callback::DownloadVersion(v) => format!("dl_ver:{}", v),
      Callback::Buy => "buy".to_string(),
      Callback::BuyPlan(plan) => format!("buy_plan:{}", plan),
      Callback::ExtendLicense => "extend_lic".to_string(),
      Callback::ExtendLicenseKey(key) => format!("ext_key:{}", key),
      Callback::ExtendPlan { key, plan } => {
        format!("ext_plan:{}:{}", key, plan)
      }
      Callback::AddFunds => "add_funds".to_string(),
      Callback::PayCryptoAmount(a) => format!("pay_amt:{}", a),
      Callback::PayCustomAmount => "pay_custom".to_string(),
      Callback::CheckPayments => "check_pay".to_string(),
      Callback::PayManual => "pay_man".to_string(),
      Callback::HaveLicense => "have_lic".to_string(),
      Callback::SetRef => "set_ref".to_string(),
      Callback::Back => "back".to_string(),
    }
  }

  pub fn from_data(data: &str) -> Option<Self> {
    match data {
      "profile" => Some(Callback::Profile),
      "license" => Some(Callback::License),
      "trial" => Some(Callback::Trial),
      "download" => Some(Callback::Download),
      "buy" => Some(Callback::Buy),
      "extend_lic" => Some(Callback::ExtendLicense),
      "add_funds" => Some(Callback::AddFunds),
      "pay_custom" => Some(Callback::PayCustomAmount),
      "check_pay" => Some(Callback::CheckPayments),
      "pay_man" => Some(Callback::PayManual),
      "have_lic" => Some(Callback::HaveLicense),
      "set_ref" => Some(Callback::SetRef),
      "back" => Some(Callback::Back),
      _ if data.starts_with("dl_ver:") => {
        Some(Callback::DownloadVersion(data[7..].to_string()))
      }
      _ if data.starts_with("pay_amt:") => {
        Some(Callback::PayCryptoAmount(data[8..].to_string()))
      }
      _ if data.starts_with("buy_plan:") => {
        Some(Callback::BuyPlan(data[9..].to_string()))
      }
      _ if data.starts_with("ext_key:") => {
        Some(Callback::ExtendLicenseKey(data[8..].to_string()))
      }
      _ if data.starts_with("ext_plan:") => {
        let parts: Vec<&str> = data[9..].splitn(2, ':').collect();
        if parts.len() == 2 {
          Some(Callback::ExtendPlan {
            key: parts[0].to_string(),
            plan: parts[1].to_string(),
          })
        } else {
          None
        }
      }
      _ => None,
    }
  }
}

pub fn main_menu(is_promo: bool) -> InlineKeyboardMarkup {
  let mut rows = vec![
    vec![InlineKeyboardButton::callback(
      "👤 My Profile",
      Callback::Profile.to_data(),
    )],
    vec![InlineKeyboardButton::callback(
      "🔑 My License",
      Callback::License.to_data(),
    )],
    vec![
      InlineKeyboardButton::callback("💳 Buy License", Callback::Buy.to_data()),
      InlineKeyboardButton::callback(
        "💵 Add Funds",
        Callback::AddFunds.to_data(),
      ),
    ],
    vec![InlineKeyboardButton::callback(
      "📥 Download Panel",
      Callback::Download.to_data(),
    )],
  ];

  if is_promo {
    rows.push(vec![InlineKeyboardButton::callback(
      "🆓 Get Free Trial",
      Callback::Trial.to_data(),
    )]);
  }

  InlineKeyboardMarkup::new(rows)
}

fn back_keyboard() -> InlineKeyboardMarkup {
  InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
    "« Back to Menu",
    Callback::Back.to_data(),
  )]])
}

/// Format balance in USDT (stored as nanoUSDT internally)
fn format_usdt(nano_usdt: i64) -> String {
  format!("{:.2} USDT", nano_usdt as f64 / NANO_USDT as f64)
}

pub async fn handle(
  app: Arc<AppState>,
  bot: ReplyBot,
  data: &str,
) -> ResponseResult<()> {
  let sv = app.sv();

  let Some(callback) = Callback::from_data(data) else {
    return Ok(());
  };

  match callback {
    Callback::Profile => {
      handle_profile_view(&sv, &bot).await?;
    }
    Callback::License => {
      handle_license_edit(&sv, &bot).await?;
    }
    Callback::Trial => {
      handle_trial_claim(&sv, &bot).await?;
    }
    Callback::Download => {
      if let Ok(keys) = sv.license.by_user(bot.chat_id.0, false).await
        && !keys.is_empty()
      {
        handle_download(&sv, &bot, &app).await?;
      } else {
        bot
          .edit_with_keyboard("You have no active license!", back_keyboard())
          .await?;
      }
    }
    Callback::Buy => {
      handle_buy_menu(&sv, &bot).await?;
    }
    Callback::BuyPlan(plan) => {
      handle_buy_plan(&sv, &bot, &plan).await?;
    }
    Callback::ExtendLicense => {
      handle_extend_license_menu(&sv, &bot).await?;
    }
    Callback::ExtendLicenseKey(key) => {
      handle_extend_license_key(&sv, &bot, &key).await?;
    }
    Callback::ExtendPlan { key, plan } => {
      handle_extend_plan(&sv, &bot, &key, &plan).await?;
    }
    Callback::AddFunds => {
      handle_add_funds(&sv, &bot, &app).await?;
    }
    Callback::PayCryptoAmount(amount) => {
      handle_pay_crypto_amount(&sv, &bot, &app, &amount).await?;
    }
    Callback::PayCustomAmount => {
      let text = "💵 <b>Custom Amount</b>\n\n\
        To add a custom amount to your balance, use the command:\n\n\
        <code>/fund AMOUNT</code>\n\n\
        Examples:\n\
        • <code>/fund 5</code> - Add 5 USDT\n\
        • <code>/fund 15.5</code> - Add 15.5 USDT\n\n\
        <i>Minimum deposit: 1 USDT</i>";

      bot.edit_with_keyboard(text, back_keyboard()).await?;
    }
    Callback::CheckPayments => {
      handle_check_payments(&sv, &bot, &app).await?;
    }
    Callback::SetRef => {
      let user = sv.user.by_id(bot.user_id).await.ok().flatten();
      let current_ref = user.as_ref().and_then(|u| u.referred_by);

      let text = format!(
        "🔗 <b>Set Referral Code</b>\n\n\
        A referral code is the User ID of someone who referred you.\n\
        When you have a referral code, you get a discount on purchases!\n\n\
        <b>Your current referral code:</b> {}\n\n\
        <b>To set/change:</b> <code>/ref USER_ID</code>\n\
        <b>To clear:</b> <code>/ref clear</code>",
        current_ref
          .map(|id| format!("<code>{}</code>", id))
          .unwrap_or_else(|| "None".to_string())
      );
      bot.edit_with_keyboard(text, back_keyboard()).await?;
    }
    Callback::PayManual => {
      let text = "👤 <b>Manual Purchase</b>\n\n\
        To purchase a license via USDT or other methods, please contact our support:\n\n\
        👉 @y_a_c_s_p\n\n\
        <i>Send a message with \"I want to buy license\"</i>";

      let kb = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::url(
          "Open Chat with Support",
          Url::parse("https://t.me/y_a_c_s_p").expect("invalid link, what???"),
        )],
        vec![InlineKeyboardButton::callback("« Back", Callback::Buy.to_data())],
      ]);

      bot.edit_with_keyboard(text, kb).await?;
    }
    Callback::Back => {
      let text = "<b>Yet Another Counter Strike Panel!</b>\n\n\
        Use the buttons below to navigate.\n\
        Read docs: https://yacsp.gitbook.io/yacsp\n\
        Contact support: @y_a_c_s_p";
      bot
        .edit_with_keyboard(text, main_menu(sv.license.is_promo_active()))
        .await?;
    }
    Callback::DownloadVersion(version) => {
      handle_download_version(&sv, &bot, &app, &version).await?;
    }
    Callback::HaveLicense => {
      let text = "🔑 <b>Link Your License</b>\n\n\
        If you already have a license key, you can link it to your account.\n\n\
        <b>To link a license:</b>\n\
        Send the command: <code>/link YOUR_LICENSE_KEY</code>\n\n\
        <b>Your User ID:</b> <code>{}</code>\n\n\
        <i>Note: When purchasing, you can provide a referrer's user ID to get a discount!</i>";
      bot
        .edit_with_keyboard(
          text.replace("{}", &bot.user_id.to_string()),
          back_keyboard(),
        )
        .await?;
    }
  }

  Ok(())
}

async fn handle_profile_view(
  sv: &Services<'_>,
  bot: &ReplyBot,
) -> ResponseResult<()> {
  let user = sv.user.by_id(bot.user_id).await.ok().flatten();

  let (reg_date, balance, role, referral_stats) = match &user {
    Some(u) => {
      let stats = sv.referral.stats(bot.user_id).await.ok();
      (utils::format_date(u.reg_date), u.balance, u.role.clone(), stats)
    }
    None => ("Unknown".into(), 0, UserRole::User, None),
  };

  let stats = sv.stats.display_stats(bot.user_id).await.ok();

  let balance_str = format_usdt(balance);
  let role_str = match role {
    UserRole::User => "User",
    UserRole::Creator => "Creator",
    UserRole::Admin => "Admin",
  };

  let mut text = format!(
    "👤 <b>My Profile</b>\n\n\
    <b>User ID:</b> <code>{}</code>\n\
    <b>Registered:</b> {}\n\
    <b>Balance:</b> {}\n\
    <b>Role:</b> {}",
    bot.user_id, reg_date, balance_str, role_str
  );

  // Show referral info for creators/admins
  if let Some(ref_stats) = referral_stats
    && ref_stats.is_active
  {
    text.push_str(&format!(
      "\n\n<b>🔗 Referral Info:</b>\n\
        Your referral code: <code>{}</code>\n\
        Commission rate: {}%\n\
        Customer discount: {}%\n\
        Total sales: {}\n\
        Total earnings: {}",
      bot.user_id,
      ref_stats.commission_rate,
      ref_stats.discount_percent,
      ref_stats.total_sales,
      format_usdt(ref_stats.total_earnings)
    ));
  }

  if let Some(s) = stats {
    text.push_str(&format!(
      "\n\n<b>📊 Farming Stats:</b>\n\
        Weekly XP: {}\n\
        Total XP: {}\n\
        Drops: {}\n\
        Runtime: {:.1}h",
      s.weekly_xp, s.total_xp, s.drops_count, s.runtime_hours
    ));

    if let Some(meta) = s.meta {
      if !meta.network.routes.is_empty() {
        text.push_str(&format!(
          "\n🌐 <b>Routes:</b> {}",
          meta.network.routes.join(", ")
        ));
      }

      if meta.performance.avg_fps > 0.0 {
        text.push_str(&format!(
          "\n🚀 <b>Perf:</b> {:.0} FPS | {} MB",
          meta.performance.avg_fps, meta.performance.avg_ram_mb
        ));
      }

      let mut states: Vec<_> = meta.states.clone().into_iter().collect();
      states.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

      if let Some((top_state, duration)) = states.first() {
        text.push_str(&format!(
          "\n⏳ <b>Top State:</b> {top_state} ({:.1}h)",
          *duration / 3600.0
        ));
      }
    }
  }

  bot.edit_with_keyboard(text, back_keyboard()).await?;

  Ok(())
}

async fn handle_license_edit(
  sv: &Services<'_>,
  bot: &ReplyBot,
) -> ResponseResult<()> {
  let now = Utc::now().naive_utc();

  match sv.license.by_user(bot.user_id, false).await {
    Ok(licenses) if !licenses.is_empty() => {
      let mut text = String::from("🔑 <b>Your Licenses:</b>\n");

      for license in licenses {
        let status = if license.expires_at > now {
          format!("⏳ {}", utils::format_duration(license.expires_at - now))
        } else {
          "❌ Expired".into()
        };

        text.push_str(&format!(
          "\n<code>{}</code>\n{} | {:?}\n",
          license.key, status, license.license_type
        ));
      }

      bot.edit_with_keyboard(text, back_keyboard()).await?;
    }
    _ => {
      bot
        .edit_with_keyboard("You have no active license!", back_keyboard())
        .await?;
    }
  }

  Ok(())
}

async fn handle_trial_claim(
  sv: &Services<'_>,
  bot: &ReplyBot,
) -> ResponseResult<()> {
  let promo_name = "first_promo";

  match sv.license.claim_promo(bot.user_id, promo_name).await {
    Ok(license) => {
      let text = format!(
        "🎉 <b>Success!</b>\n\n\
        Here is your FREE week license:\n\
        <code>{}</code>\n\n\
        Download the software using the Download button!",
        license.key
      );
      bot.reply_with_keyboard(text, back_keyboard()).await?;
    }
    Err(e) => {
      let msg = match e {
        Error::Promo(Promo::Inactive) => "Promo is not active right now.",
        Error::Promo(Promo::Claimed) => "You have already claimed this promo",
        _ => "An error occurred.",
      };
      bot.reply_with_keyboard(msg, back_keyboard()).await?;
    }
  }

  Ok(())
}

async fn handle_download(
  sv: &Services<'_>,
  bot: &ReplyBot,
  app: &AppState,
) -> ResponseResult<()> {
  let builds = sv.build.active().await.unwrap_or_default();

  if builds.is_empty() {
    bot
      .edit_with_keyboard(
        "❌ No builds available yet. Contact support.",
        back_keyboard(),
      )
      .await?;
    return Ok(());
  }

  // If only one version available, download directly
  if builds.len() == 1 {
    return handle_download_version(sv, bot, app, &builds[0].version).await;
  }

  // Multiple versions - show selection menu
  let mut rows = Vec::new();
  for build in &builds {
    let label = if Some(build.id) == builds.first().map(|b| b.id) {
      format!("📥 v{} (latest)", build.version)
    } else {
      format!("📥 v{}", build.version)
    };
    rows.push(vec![InlineKeyboardButton::callback(
      label,
      Callback::DownloadVersion(build.version.clone()).to_data(),
    )]);
  }
  rows.push(vec![InlineKeyboardButton::callback(
    "« Back to Menu",
    Callback::Back.to_data(),
  )]);

  let text = "📥 <b>Select Version</b>\n\n\
    Choose which version to download:";

  bot.edit_with_keyboard(text, InlineKeyboardMarkup::new(rows)).await?;

  Ok(())
}

async fn handle_download_version(
  sv: &Services<'_>,
  bot: &ReplyBot,
  app: &AppState,
  version: &str,
) -> ResponseResult<()> {
  match sv.build.by_version(version).await {
    Ok(Some(build)) if build.is_active => {
      let path = Path::new(&build.file_path);
      if path.exists() {
        let token = app.create_download_token(&build.version);
        let download_url =
          format!("{}/api/download?token={}", app.config.base_url, token);

        let text = format!(
          "<b>YACS Panel v{}</b>\n\n\
          {}\n\n\
          📥 <a href=\"{}\">Click here to download</a>\n\n\
          <i>⚠️ Link expires in 10 minutes</i>",
          build.version,
          build.changelog.as_deref().unwrap_or(""),
          download_url
        );

        bot.edit_with_keyboard(text, back_keyboard()).await?;
      } else {
        bot
          .edit_with_keyboard(
            "❌ Build file not found. Contact support.",
            back_keyboard(),
          )
          .await?;
      }
    }
    _ => {
      bot
        .edit_with_keyboard(
          "❌ Build not available. Contact support.",
          back_keyboard(),
        )
        .await?;
    }
  }

  Ok(())
}

/// Price constants in USDT
const MONTH_PRICE: f64 = 10.0;
const QUARTER_PRICE: f64 = 25.0;

/// Nano USDT price constants
const MONTH_PRICE_NANO: i64 = 10 * NANO_USDT;
const QUARTER_PRICE_NANO: i64 = 25 * NANO_USDT;

async fn handle_buy_menu(
  sv: &Services<'_>,
  bot: &ReplyBot,
) -> ResponseResult<()> {
  let user = sv.user.by_id(bot.user_id).await.ok().flatten();
  let balance = user.as_ref().map(|u| u.balance).unwrap_or(0);
  let referred_by = user.as_ref().and_then(|u| u.referred_by);
  let balance_str = format_usdt(balance);

  // Get discount if user has a referrer
  let discount_percent = if let Some(ref_id) = referred_by {
    sv.referral.stats(ref_id).await.map(|s| s.discount_percent).unwrap_or(0)
  } else {
    0
  };

  let (month_price, quarter_price) = if discount_percent > 0 {
    let month_discounted =
      MONTH_PRICE * (100 - discount_percent) as f64 / 100.0;
    let quarter_discounted =
      QUARTER_PRICE * (100 - discount_percent) as f64 / 100.0;
    (month_discounted, quarter_discounted)
  } else {
    (MONTH_PRICE, QUARTER_PRICE)
  };

  let month_nano = (month_price * NANO_USDT as f64) as i64;
  let quarter_nano = (quarter_price * NANO_USDT as f64) as i64;

  let can_buy_month = balance >= month_nano;
  let can_buy_quarter = balance >= quarter_nano;

  let mut text = format!(
    "💳 <b>Buy License</b>\n\n\
    <b>Your Balance:</b> {}\n\n\
    <b>Pricing:</b>\n",
    balance_str
  );

  if discount_percent > 0 {
    text.push_str(&format!(
      "• 1 Month: <s>10.00</s> <b>{:.2} USDT</b> ({}% off)\n\
       • 3 Months: <s>25.00</s> <b>{:.2} USDT</b> ({}% off)\n\n\
       <i>🎉 Discount from referral code <code>{}</code></i>\n",
      month_price,
      discount_percent,
      quarter_price,
      discount_percent,
      referred_by.unwrap()
    ));
  } else {
    text.push_str(&format!(
      "• 1 Month: <b>{:.2} USDT</b>\n\
       • 3 Months: <b>{:.2} USDT</b>\n",
      month_price, quarter_price
    ));
  }

  if can_buy_month {
    text.push_str("\n<i>Select a plan to purchase with your balance:</i>");
  } else {
    text.push_str(&format!(
      "\n<i>💡 You need {} more to buy 1 month license.</i>",
      format_usdt(month_nano - balance)
    ));
  }

  if referred_by.is_none() {
    text.push_str("\n\n<i>💡 Tip: Set a referral code to get a discount!</i>");
  }

  let mut rows = Vec::new();

  // Buy buttons (only enabled if sufficient balance)
  if can_buy_month {
    rows.push(vec![InlineKeyboardButton::callback(
      format!("📅 1 Month ({:.2} USDT)", month_price),
      Callback::BuyPlan("month".to_string()).to_data(),
    )]);
  }
  if can_buy_quarter {
    rows.push(vec![InlineKeyboardButton::callback(
      format!("📅 3 Months ({:.2} USDT)", quarter_price),
      Callback::BuyPlan("quarter".to_string()).to_data(),
    )]);
  }

  // Extend existing license button
  rows.push(vec![InlineKeyboardButton::callback(
    "🔄 Extend License",
    Callback::ExtendLicense.to_data(),
  )]);

  // Add funds button
  rows.push(vec![InlineKeyboardButton::callback(
    "💵 Add Funds",
    Callback::AddFunds.to_data(),
  )]);

  if referred_by.is_none() {
    rows.push(vec![InlineKeyboardButton::callback(
      "🔗 Set Referral Code",
      Callback::SetRef.to_data(),
    )]);
  }

  // Other options
  rows.push(vec![
    InlineKeyboardButton::callback("👤 Manual", Callback::PayManual.to_data()),
    InlineKeyboardButton::callback(
      "🔑 Link Key",
      Callback::HaveLicense.to_data(),
    ),
  ]);

  rows.push(vec![InlineKeyboardButton::callback(
    "« Back to Menu",
    Callback::Back.to_data(),
  )]);

  bot.edit_with_keyboard(&text, InlineKeyboardMarkup::new(rows)).await?;
  Ok(())
}

async fn handle_buy_plan(
  sv: &Services<'_>,
  bot: &ReplyBot,
  plan: &str,
) -> ResponseResult<()> {
  let user = sv.user.by_id(bot.user_id).await.ok().flatten();
  let balance = user.as_ref().map(|u| u.balance).unwrap_or(0);
  let referred_by = user.as_ref().and_then(|u| u.referred_by);

  // Get discount if user has a referrer
  let discount_percent = if let Some(ref_id) = referred_by {
    sv.referral.stats(ref_id).await.map(|s| s.discount_percent).unwrap_or(0)
  } else {
    0
  };

  let (price, days, plan_name) = match plan {
    "month" => {
      let price = if discount_percent > 0 {
        MONTH_PRICE_NANO * (100 - discount_percent) as i64 / 100
      } else {
        MONTH_PRICE_NANO
      };
      (price, 30u64, "1 Month")
    }
    "quarter" => {
      let price = if discount_percent > 0 {
        QUARTER_PRICE_NANO * (100 - discount_percent) as i64 / 100
      } else {
        QUARTER_PRICE_NANO
      };
      (price, 90u64, "3 Months")
    }
    _ => {
      bot.edit_with_keyboard("❌ Invalid plan.", back_keyboard()).await?;
      return Ok(());
    }
  };

  if balance < price {
    let needed = price - balance;
    let text = format!(
      "❌ <b>Insufficient Balance</b>\n\n\
      <b>Required:</b> {}\n\
      <b>Your balance:</b> {}\n\
      <b>Needed:</b> {}\n\n\
      <i>Add funds to your balance to purchase this plan.</i>",
      format_usdt(price),
      format_usdt(balance),
      format_usdt(needed)
    );
    let kb = InlineKeyboardMarkup::new(vec![
      vec![InlineKeyboardButton::callback(
        "💵 Add Funds",
        Callback::AddFunds.to_data(),
      )],
      vec![InlineKeyboardButton::callback("« Back", Callback::Buy.to_data())],
    ]);
    bot.edit_with_keyboard(text, kb).await?;
    return Ok(());
  }

  // Purchase the license
  match sv
    .balance
    .spend(
      bot.user_id,
      price,
      Some(format!("License purchase: {}", plan_name)),
      referred_by,
    )
    .await
  {
    Ok(new_balance) => {
      // If user was referred, process referral commission
      if let Some(referrer_id) = referred_by {
        let _ = sv.referral.record_sale(referrer_id, price).await;
        // Add commission to referrer's balance
        let referrer_user = sv.user.by_id(referrer_id).await.ok().flatten();
        if let Some(referrer) = referrer_user {
          let commission = price * referrer.commission_rate as i64 / 100;
          let _ = sv
            .balance
            .add_referral_bonus(referrer_id, commission, bot.user_id)
            .await;
        }
      }

      // Generate license
      match sv
        .license
        .create(bot.user_id, crate::entity::license::LicenseType::Pro, days)
        .await
      {
        Ok(license) => {
          let text = format!(
            "✅ <b>Purchase Successful!</b>\n\n\
            <b>Plan:</b> {}\n\
            <b>License Key:</b> <code>{}</code>\n\
            <b>Expires:</b> {}\n\n\
            <b>New Balance:</b> {}\n\n\
            <i>You can now download the panel!</i>",
            plan_name,
            license.key,
            crate::utils::format_date(license.expires_at),
            format_usdt(new_balance)
          );
          let kb = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
              "📥 Download Panel",
              Callback::Download.to_data(),
            )],
            vec![InlineKeyboardButton::callback(
              "« Back to Menu",
              Callback::Back.to_data(),
            )],
          ]);
          bot.edit_with_keyboard(text, kb).await?;
        }
        Err(e) => {
          // Refund on failure
          let _ = sv
            .balance
            .deposit(
              bot.user_id,
              price,
              Some("Refund: license creation failed".into()),
            )
            .await;
          let text =
            format!("❌ Failed to create license: {}", e.user_message());
          bot.edit_with_keyboard(text, back_keyboard()).await?;
        }
      }
    }
    Err(e) => {
      let text = format!("❌ Failed to process payment: {}", e.user_message());
      bot.edit_with_keyboard(text, back_keyboard()).await?;
    }
  }

  Ok(())
}

async fn handle_add_funds(
  sv: &Services<'_>,
  bot: &ReplyBot,
  app: &AppState,
) -> ResponseResult<()> {
  let user = sv.user.by_id(bot.user_id).await.ok().flatten();
  let balance = user.as_ref().map(|u| u.balance).unwrap_or(0);
  let referred_by = user.as_ref().and_then(|u| u.referred_by);

  // Get discount if user has a referrer
  let discount_percent = if let Some(ref_id) = referred_by {
    sv.referral.stats(ref_id).await.map(|s| s.discount_percent).unwrap_or(0)
  } else {
    0
  };

  let (month_price, quarter_price) = if discount_percent > 0 {
    let month_discounted =
      MONTH_PRICE * (100 - discount_percent) as f64 / 100.0;
    let quarter_discounted =
      QUARTER_PRICE * (100 - discount_percent) as f64 / 100.0;
    (month_discounted, quarter_discounted)
  } else {
    (MONTH_PRICE, QUARTER_PRICE)
  };

  let has_cryptobot = app.cryptobot.is_some();

  let pending =
    sv.payment.pending_by_user(bot.user_id).await.unwrap_or_default();
  let pending_count = pending.len();

  let mut text = format!(
    "💵 <b>Add Funds</b>\n\n\
    <b>Your Balance:</b> {}\n\n\
    <b>Quick amounts:</b>\n\
    • {:.2} USDT (1 month license)\n\
    • {:.2} USDT (3 month license)\n",
    format_usdt(balance),
    month_price,
    quarter_price
  );

  if discount_percent > 0 {
    text.push_str(&format!(
      "\n<i>🎉 {}% discount available from referral!</i>\n",
      discount_percent
    ));
  }

  if pending_count > 0 {
    text.push_str(&format!(
      "\n<i>⏳ You have {} pending payment(s).</i>\n",
      pending_count
    ));
  }

  if has_cryptobot {
    text.push_str(
      "\n<i>Select an amount or use /fund AMOUNT for custom amounts.</i>",
    );
  } else {
    text.push_str(
      "\n<i>⚠️ Automatic payments are being configured.\nContact support for manual deposits.</i>",
    );
  }

  let mut rows = Vec::new();

  if has_cryptobot {
    rows.push(vec![
      InlineKeyboardButton::callback(
        format!("{:.2} USDT", month_price),
        Callback::PayCryptoAmount(format!("{:.2}", month_price)).to_data(),
      ),
      InlineKeyboardButton::callback(
        format!("{:.2} USDT", quarter_price),
        Callback::PayCryptoAmount(format!("{:.2}", quarter_price)).to_data(),
      ),
    ]);
    rows.push(vec![InlineKeyboardButton::callback(
      "💵 Custom Amount",
      Callback::PayCustomAmount.to_data(),
    )]);
  }

  if pending_count > 0 {
    rows.push(vec![InlineKeyboardButton::callback(
      "🔄 Check Payments",
      Callback::CheckPayments.to_data(),
    )]);
  }

  if !has_cryptobot {
    rows.push(vec![InlineKeyboardButton::url(
      "📞 Contact Support",
      Url::parse("https://t.me/y_a_c_s_p").expect("invalid url"),
    )]);
  }

  rows.push(vec![InlineKeyboardButton::callback(
    "« Back to Menu",
    Callback::Back.to_data(),
  )]);

  bot.edit_with_keyboard(text, InlineKeyboardMarkup::new(rows)).await?;
  Ok(())
}

async fn handle_pay_crypto_amount(
  sv: &Services<'_>,
  bot: &ReplyBot,
  app: &AppState,
  amount: &str,
) -> ResponseResult<()> {
  let Some(cryptobot) = &app.cryptobot else {
    bot
      .edit_with_keyboard(
        "❌ CryptoBot payments are not configured. Contact support.",
        back_keyboard(),
      )
      .await?;
    return Ok(());
  };

  let amount_usdt: f64 = match amount.parse() {
    Ok(a) => a,
    Err(_) => {
      bot.edit_with_keyboard("❌ Invalid amount.", back_keyboard()).await?;
      return Ok(());
    }
  };

  let user = sv.user.by_id(bot.user_id).await.ok().flatten();
  let referred_by = user.as_ref().and_then(|u| u.referred_by);

  // Create invoice
  match cryptobot
    .create_deposit_invoice(bot.user_id, amount_usdt, referred_by)
    .await
  {
    Ok(invoice) => {
      // Save pending invoice for later polling
      let _ = sv
        .payment
        .save_pending(invoice.invoice_id, bot.user_id, amount_usdt, referred_by)
        .await;

      let text = format!(
        "💳 <b>Payment Invoice Created</b>\n\n\
        <b>Amount:</b> {} USDT\n\n\
        Click the button below to pay via CryptoBot.\n\
        The invoice expires in 1 hour.\n\n\
        <i>After payment, click \"Check Payments\" to update your balance.</i>",
        amount
      );

      let kb = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::url(
          "💵 Pay Now",
          Url::parse(&invoice.bot_invoice_url).expect("invalid invoice url"),
        )],
        vec![InlineKeyboardButton::callback(
          "🔄 Check Payments",
          Callback::CheckPayments.to_data(),
        )],
        vec![InlineKeyboardButton::callback(
          "« Back",
          Callback::AddFunds.to_data(),
        )],
      ]);

      bot.edit_with_keyboard(text, kb).await?;
    }
    Err(e) => {
      let text = format!(
        "❌ Failed to create invoice: {}\n\n\
        Please try again or contact support.",
        e.user_message()
      );
      let kb =
        InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
          "« Back",
          Callback::AddFunds.to_data(),
        )]]);
      bot.edit_with_keyboard(text, kb).await?;
    }
  }

  Ok(())
}

async fn handle_check_payments(
  sv: &Services<'_>,
  bot: &ReplyBot,
  app: &AppState,
) -> ResponseResult<()> {
  let Some(cryptobot) = &app.cryptobot else {
    bot
      .edit_with_keyboard(
        "❌ Payment verification is not configured.",
        back_keyboard(),
      )
      .await?;
    return Ok(());
  };

  // Check for paid invoices and process them
  match sv.payment.check_and_process(cryptobot, bot.user_id).await {
    Ok(results) if !results.is_empty() => {
      let total: i64 = results.iter().map(|r| r.amount_nano).sum();
      let total_str = format_usdt(total);

      let text = format!(
        "✅ <b>Payment Received!</b>\n\n\
        <b>{}</b> has been added to your balance.\n\n\
        <i>Use your balance to purchase licenses in the Buy menu.</i>",
        total_str
      );

      let kb = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
          "💳 Buy License",
          Callback::Buy.to_data(),
        )],
        vec![InlineKeyboardButton::callback(
          "« Back to Menu",
          Callback::Back.to_data(),
        )],
      ]);

      bot.edit_with_keyboard(text, kb).await?;
    }
    Ok(_) => {
      // No paid invoices found
      let pending =
        sv.payment.pending_by_user(bot.user_id).await.unwrap_or_default();

      let text = if pending.is_empty() {
        "📭 <b>No Pending Payments</b>\n\n\
        You have no pending invoices.\n\
        Create a new invoice to add funds to your balance."
          .to_string()
      } else {
        format!(
          "⏳ <b>Waiting for Payment</b>\n\n\
          You have {} pending invoice(s).\n\
          Complete the payment in CryptoBot, then click \"Check Payments\" again.\n\n\
          <i>Invoices expire after 1 hour.</i>",
          pending.len()
        )
      };

      let mut rows = Vec::new();
      if !pending.is_empty() {
        rows.push(vec![InlineKeyboardButton::callback(
          "🔄 Check Payments",
          Callback::CheckPayments.to_data(),
        )]);
      }
      rows.push(vec![InlineKeyboardButton::callback(
        "💵 Add Funds",
        Callback::AddFunds.to_data(),
      )]);
      rows.push(vec![InlineKeyboardButton::callback(
        "« Back to Menu",
        Callback::Back.to_data(),
      )]);

      bot.edit_with_keyboard(text, InlineKeyboardMarkup::new(rows)).await?;
    }
    Err(e) => {
      let text = format!(
        "❌ Failed to check payments: {}\n\n\
        Please try again later.",
        e.user_message()
      );
      bot
        .edit_with_keyboard(
          text,
          InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback(
              "🔄 Try Again",
              Callback::CheckPayments.to_data(),
            ),
          ]]),
        )
        .await?;
    }
  }

  Ok(())
}

async fn handle_extend_license_menu(
  sv: &Services<'_>,
  bot: &ReplyBot,
) -> ResponseResult<()> {
  let licenses =
    sv.license.by_user(bot.user_id, false).await.unwrap_or_default();

  if licenses.is_empty() {
    let text = "❌ <b>No Licenses Found</b>\n\n\
      You don't have any licenses to extend.\n\
      Purchase a new license first.";
    let kb = InlineKeyboardMarkup::new(vec![
      vec![InlineKeyboardButton::callback(
        "💳 Buy License",
        Callback::Buy.to_data(),
      )],
      vec![InlineKeyboardButton::callback("« Back", Callback::Buy.to_data())],
    ]);
    bot.edit_with_keyboard(text, kb).await?;
    return Ok(());
  }

  let user = sv.user.by_id(bot.user_id).await.ok().flatten();
  let balance = user.as_ref().map(|u| u.balance).unwrap_or(0);
  let now = Utc::now().naive_utc();

  let mut text = format!(
    "🔄 <b>Extend License</b>\n\n\
    <b>Your Balance:</b> {}\n\n\
    <b>Select a license to extend:</b>\n",
    format_usdt(balance)
  );

  let mut rows = Vec::new();
  for license in &licenses {
    let status = if license.expires_at > now {
      format!("⏳ {}", crate::utils::format_duration(license.expires_at - now))
    } else {
      "❌ Expired".into()
    };

    text.push_str(&format!(
      "\n<code>{}</code>\n{}\n",
      &license.key[..8],
      status
    ));

    let btn_text = format!("🔑 {}...", &license.key[..8]);
    rows.push(vec![InlineKeyboardButton::callback(
      btn_text,
      Callback::ExtendLicenseKey(license.key.clone()).to_data(),
    )]);
  }

  rows.push(vec![InlineKeyboardButton::callback(
    "« Back",
    Callback::Buy.to_data(),
  )]);

  bot.edit_with_keyboard(text, InlineKeyboardMarkup::new(rows)).await?;
  Ok(())
}

async fn handle_extend_license_key(
  sv: &Services<'_>,
  bot: &ReplyBot,
  key: &str,
) -> ResponseResult<()> {
  let license = match sv.license.by_key(key).await {
    Ok(Some(l)) if l.tg_user_id == bot.user_id => l,
    _ => {
      bot.edit_with_keyboard("❌ License not found.", back_keyboard()).await?;
      return Ok(());
    }
  };

  let user = sv.user.by_id(bot.user_id).await.ok().flatten();
  let balance = user.as_ref().map(|u| u.balance).unwrap_or(0);
  let referred_by = user.as_ref().and_then(|u| u.referred_by);
  let now = Utc::now().naive_utc();

  let discount_percent = if let Some(ref_id) = referred_by {
    sv.referral.stats(ref_id).await.map(|s| s.discount_percent).unwrap_or(0)
  } else {
    0
  };

  let (month_price, quarter_price) = if discount_percent > 0 {
    (
      MONTH_PRICE * (100 - discount_percent) as f64 / 100.0,
      QUARTER_PRICE * (100 - discount_percent) as f64 / 100.0,
    )
  } else {
    (MONTH_PRICE, QUARTER_PRICE)
  };

  let month_nano = (month_price * NANO_USDT as f64) as i64;
  let quarter_nano = (quarter_price * NANO_USDT as f64) as i64;

  let status = if license.expires_at > now {
    format!("⏳ {}", crate::utils::format_duration(license.expires_at - now))
  } else {
    "❌ Expired".into()
  };

  let mut text = format!(
    "🔄 <b>Extend License</b>\n\n\
    <b>License:</b> <code>{}</code>\n\
    <b>Status:</b> {}\n\
    <b>Expires:</b> {}\n\n\
    <b>Your Balance:</b> {}\n\n\
    <b>Extension Pricing:</b>\n",
    license.key,
    status,
    crate::utils::format_date(license.expires_at),
    format_usdt(balance)
  );

  if discount_percent > 0 {
    text.push_str(&format!(
      "• +1 Month: <s>10.00</s> <b>{:.2} USDT</b> ({}% off)\n\
       • +3 Months: <s>25.00</s> <b>{:.2} USDT</b> ({}% off)\n",
      month_price, discount_percent, quarter_price, discount_percent
    ));
  } else {
    text.push_str(&format!(
      "• +1 Month: <b>{:.2} USDT</b>\n\
       • +3 Months: <b>{:.2} USDT</b>\n",
      month_price, quarter_price
    ));
  }

  let can_buy_month = balance >= month_nano;
  let can_buy_quarter = balance >= quarter_nano;

  if !can_buy_month {
    text.push_str(&format!(
      "\n<i>💡 You need {} more to extend by 1 month.</i>",
      format_usdt(month_nano - balance)
    ));
  }

  let mut rows = Vec::new();

  if can_buy_month {
    rows.push(vec![InlineKeyboardButton::callback(
      format!("+1 Month ({:.2} USDT)", month_price),
      Callback::ExtendPlan { key: key.to_string(), plan: "month".to_string() }
        .to_data(),
    )]);
  }
  if can_buy_quarter {
    rows.push(vec![InlineKeyboardButton::callback(
      format!("+3 Months ({:.2} USDT)", quarter_price),
      Callback::ExtendPlan {
        key: key.to_string(),
        plan: "quarter".to_string(),
      }
      .to_data(),
    )]);
  }

  rows.push(vec![InlineKeyboardButton::callback(
    "💵 Add Funds",
    Callback::AddFunds.to_data(),
  )]);
  rows.push(vec![InlineKeyboardButton::callback(
    "« Back",
    Callback::ExtendLicense.to_data(),
  )]);

  bot.edit_with_keyboard(text, InlineKeyboardMarkup::new(rows)).await?;
  Ok(())
}

async fn handle_extend_plan(
  sv: &Services<'_>,
  bot: &ReplyBot,
  key: &str,
  plan: &str,
) -> ResponseResult<()> {
  let license = match sv.license.by_key(key).await {
    Ok(Some(l)) if l.tg_user_id == bot.user_id => l,
    _ => {
      bot.edit_with_keyboard("❌ License not found.", back_keyboard()).await?;
      return Ok(());
    }
  };

  let user = sv.user.by_id(bot.user_id).await.ok().flatten();
  let balance = user.as_ref().map(|u| u.balance).unwrap_or(0);
  let referred_by = user.as_ref().and_then(|u| u.referred_by);

  let discount_percent = if let Some(ref_id) = referred_by {
    sv.referral.stats(ref_id).await.map(|s| s.discount_percent).unwrap_or(0)
  } else {
    0
  };

  let (price, days, plan_name) = match plan {
    "month" => {
      let price = if discount_percent > 0 {
        MONTH_PRICE_NANO * (100 - discount_percent) as i64 / 100
      } else {
        MONTH_PRICE_NANO
      };
      (price, 30u64, "1 Month")
    }
    "quarter" => {
      let price = if discount_percent > 0 {
        QUARTER_PRICE_NANO * (100 - discount_percent) as i64 / 100
      } else {
        QUARTER_PRICE_NANO
      };
      (price, 90u64, "3 Months")
    }
    _ => {
      bot.edit_with_keyboard("❌ Invalid plan.", back_keyboard()).await?;
      return Ok(());
    }
  };

  if balance < price {
    let needed = price - balance;
    let text = format!(
      "❌ <b>Insufficient Balance</b>\n\n\
      <b>Required:</b> {}\n\
      <b>Your balance:</b> {}\n\
      <b>Needed:</b> {}\n\n\
      <i>Add funds to your balance to extend this license.</i>",
      format_usdt(price),
      format_usdt(balance),
      format_usdt(needed)
    );
    let kb = InlineKeyboardMarkup::new(vec![
      vec![InlineKeyboardButton::callback(
        "💵 Add Funds",
        Callback::AddFunds.to_data(),
      )],
      vec![InlineKeyboardButton::callback(
        "« Back",
        Callback::ExtendLicenseKey(key.to_string()).to_data(),
      )],
    ]);
    bot.edit_with_keyboard(text, kb).await?;
    return Ok(());
  }

  match sv
    .balance
    .spend(
      bot.user_id,
      price,
      Some(format!("License extension: {} for {}", plan_name, &key[..8])),
      referred_by,
    )
    .await
  {
    Ok(new_balance) => {
      if let Some(referrer_id) = referred_by {
        let _ = sv.referral.record_sale(referrer_id, price).await;
        let referrer_user = sv.user.by_id(referrer_id).await.ok().flatten();
        if let Some(referrer) = referrer_user {
          let commission = price * referrer.commission_rate as i64 / 100;
          let _ = sv
            .balance
            .add_referral_bonus(referrer_id, commission, bot.user_id)
            .await;
        }
      }

      let duration = Duration::from_secs(days * 24 * 60 * 60);
      match sv.license.expires(key, duration).await {
        Ok(new_exp) => {
          let text = format!(
            "✅ <b>License Extended!</b>\n\n\
            <b>License:</b> <code>{}</code>\n\
            <b>Added:</b> {}\n\
            <b>New Expiry:</b> {}\n\n\
            <b>New Balance:</b> {}",
            license.key,
            plan_name,
            crate::utils::format_date(new_exp),
            format_usdt(new_balance)
          );
          let kb = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
              "📥 Download Panel",
              Callback::Download.to_data(),
            )],
            vec![InlineKeyboardButton::callback(
              "« Back to Menu",
              Callback::Back.to_data(),
            )],
          ]);
          bot.edit_with_keyboard(text, kb).await?;
        }
        Err(e) => {
          let _ = sv
            .balance
            .deposit(
              bot.user_id,
              price,
              Some("Refund: license extension failed".into()),
            )
            .await;
          let text =
            format!("❌ Failed to extend license: {}", e.user_message());
          bot.edit_with_keyboard(text, back_keyboard()).await?;
        }
      }
    }
    Err(e) => {
      let text = format!("❌ Failed to process payment: {}", e.user_message());
      bot.edit_with_keyboard(text, back_keyboard()).await?;
    }
  }

  Ok(())
}
