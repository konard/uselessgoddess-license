use std::{path::Path, sync::Arc};

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
  PayCrypto,
  PayCryptoAmount(String),
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
      Callback::PayCrypto => "pay_crypto".to_string(),
      Callback::PayCryptoAmount(a) => format!("pay_amt:{}", a),
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
      "pay_crypto" => Some(Callback::PayCrypto),
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
    vec![InlineKeyboardButton::callback(
      "💳 Buy License",
      Callback::Buy.to_data(),
    )],
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

fn payment_method_menu() -> InlineKeyboardMarkup {
  InlineKeyboardMarkup::new(vec![
    vec![InlineKeyboardButton::callback(
      "💳 Pay with Crypto",
      Callback::PayCrypto.to_data(),
    )],
    vec![InlineKeyboardButton::callback(
      "👤 Manual Purchase",
      Callback::PayManual.to_data(),
    )],
    vec![InlineKeyboardButton::callback(
      "🔑 I Have a License",
      Callback::HaveLicense.to_data(),
    )],
    vec![InlineKeyboardButton::callback(
      "« Back to Menu",
      Callback::Back.to_data(),
    )],
  ])
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
    Callback::PayCrypto => {
      handle_pay_crypto(&sv, &bot, &app).await?;
    }
    Callback::PayCryptoAmount(amount) => {
      handle_pay_crypto_amount(&sv, &bot, &app, &amount).await?;
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

  let mut text = format!(
    "💳 <b>Purchase License</b>\n\n\
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

  text.push_str("\nSelect a payment method below.");

  if referred_by.is_none() {
    text.push_str("\n\n<i>💡 Tip: Set a referral code to get a discount!</i>");
  }

  let mut menu = payment_method_menu();
  if referred_by.is_none() {
    // Add set referral button if user doesn't have one
    menu.inline_keyboard.insert(
      0,
      vec![InlineKeyboardButton::callback(
        "🔗 Set Referral Code",
        Callback::SetRef.to_data(),
      )],
    );
  }

  bot.edit_with_keyboard(&text, menu).await?;
  Ok(())
}

async fn handle_pay_crypto(
  sv: &Services<'_>,
  bot: &ReplyBot,
  app: &AppState,
) -> ResponseResult<()> {
  let user = sv.user.by_id(bot.user_id).await.ok().flatten();
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

  let mut text = format!(
    "💳 <b>Pay with Crypto</b>\n\n\
    Select the amount to deposit:\n\n\
    • 1 Month License: <b>{:.2} USDT</b>\n\
    • 3 Month License: <b>{:.2} USDT</b>\n",
    month_price, quarter_price
  );

  if discount_percent > 0 {
    text.push_str(&format!(
      "\n<i>🎉 {}% discount applied from referral!</i>\n",
      discount_percent
    ));
  }

  if has_cryptobot {
    text.push_str("\nClick a button below to generate a payment link:");
  } else {
    text.push_str(
      "\n<i>⚠️ Automatic payments are being configured.\nPlease contact support for manual deposits.</i>",
    );
  }

  let mut rows = Vec::new();

  if has_cryptobot {
    rows.push(vec![InlineKeyboardButton::callback(
      format!("💵 Pay {:.2} USDT (1 Month)", month_price),
      Callback::PayCryptoAmount(format!("{:.2}", month_price)).to_data(),
    )]);
    rows.push(vec![InlineKeyboardButton::callback(
      format!("💵 Pay {:.2} USDT (3 Months)", quarter_price),
      Callback::PayCryptoAmount(format!("{:.2}", quarter_price)).to_data(),
    )]);
  } else {
    rows.push(vec![InlineKeyboardButton::url(
      "📞 Contact Support",
      Url::parse("https://t.me/y_a_c_s_p").expect("invalid url"),
    )]);
  }

  rows.push(vec![InlineKeyboardButton::callback(
    "« Back",
    Callback::Buy.to_data(),
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
          Callback::PayCrypto.to_data(),
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
          Callback::PayCrypto.to_data(),
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
        "💳 Create New Invoice",
        Callback::PayCrypto.to_data(),
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
