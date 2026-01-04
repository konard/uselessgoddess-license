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
  sv::referral::CENTS_PER_DOLLAR,
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
  PayManual,
  HaveLicense,
  Balance,
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
      Callback::PayManual => "pay_man".to_string(),
      Callback::HaveLicense => "have_lic".to_string(),
      Callback::Balance => "balance".to_string(),
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
      "pay_man" => Some(Callback::PayManual),
      "have_lic" => Some(Callback::HaveLicense),
      "balance" => Some(Callback::Balance),
      "back" => Some(Callback::Back),
      _ if data.starts_with("dl_ver:") => {
        Some(Callback::DownloadVersion(data[7..].to_string()))
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
      let user = sv.user.by_id(bot.user_id).await.ok().flatten();
      let balance = user.map(|u| u.balance).unwrap_or(0);
      let balance_str = format!("${:.2}", balance as f64 / CENTS_PER_DOLLAR as f64);

      let text = format!(
        "💳 <b>Purchase License</b>\n\n\
        <b>Your Balance:</b> {}\n\n\
        Select a payment method below.",
        balance_str
      );
      bot.edit_with_keyboard(&text, payment_method_menu()).await?;
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
        <i>Note: Use a referral code when purchasing to get a discount!</i>";
      bot
        .edit_with_keyboard(
          text.replace("{}", &bot.user_id.to_string()),
          back_keyboard(),
        )
        .await?;
    }
    Callback::Balance => {
      handle_balance_view(&sv, &bot).await?;
    }
  }

  Ok(())
}

async fn handle_profile_view(
  sv: &Services<'_>,
  bot: &ReplyBot,
) -> ResponseResult<()> {
  let user = sv.user.by_id(bot.user_id).await.ok().flatten();

  let (reg_date, balance, role) = match &user {
    Some(u) => (
      utils::format_date(u.reg_date),
      u.balance,
      u.role.clone(),
    ),
    None => ("Unknown".into(), 0, UserRole::User),
  };

  let stats = sv.stats.display_stats(bot.user_id).await.ok();

  let balance_str = format!("${:.2}", balance as f64 / CENTS_PER_DOLLAR as f64);
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

  if let Some(s) = stats {
    // Базовая стата
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

async fn handle_balance_view(
  sv: &Services<'_>,
  bot: &ReplyBot,
) -> ResponseResult<()> {
  let user = sv.user.by_id(bot.user_id).await.ok().flatten();

  let (balance, role, referral_codes) = match user {
    Some(u) => {
      let codes = sv.referral.by_owner(bot.user_id).await.unwrap_or_default();
      (u.balance, u.role, codes)
    }
    None => (0, UserRole::User, vec![]),
  };

  let balance_str = format!("${:.2}", balance as f64 / CENTS_PER_DOLLAR as f64);

  let role_str = match role {
    UserRole::User => "User",
    UserRole::Creator => "Creator",
    UserRole::Admin => "Admin",
  };

  let can_withdraw = role == UserRole::Creator || role == UserRole::Admin;

  let mut text = format!(
    "💰 <b>My Balance</b>\n\n\
    <b>Balance:</b> {}\n\
    <b>Role:</b> {}\n",
    balance_str, role_str
  );

  if can_withdraw {
    text.push_str("\n<i>💸 As a creator, you can withdraw your balance to crypto.\nContact @y_a_c_s_p to process withdrawal.</i>\n");
  } else {
    text.push_str("\n<i>💡 Your balance can be used for license purchases.</i>\n");
  }

  if !referral_codes.is_empty() {
    text.push_str("\n<b>🔗 Your Referral Codes:</b>\n");
    for code in referral_codes {
      let status = if code.is_active { "✅" } else { "❌" };
      let earnings = code.total_earnings as f64 / CENTS_PER_DOLLAR as f64;
      text.push_str(&format!(
        "{} <code>{}</code> - {}% rate, {} sales, ${:.2} earned\n",
        status, code.code, code.commission_rate, code.total_sales, earnings
      ));
    }
  }

  let transactions =
    sv.balance.transactions(bot.user_id, 5).await.unwrap_or_default();
  if !transactions.is_empty() {
    text.push_str("\n<b>📜 Recent Transactions:</b>\n");
    for tx in transactions {
      let sign = if tx.amount > 0 { "+" } else { "" };
      let amount_str =
        format!("{}${:.2}", sign, tx.amount as f64 / CENTS_PER_DOLLAR as f64);
      text.push_str(&format!(
        "{} {:?} - {}\n",
        utils::format_date(tx.created_at),
        tx.tx_type,
        amount_str
      ));
    }
  }

  bot.edit_with_keyboard(text, back_keyboard()).await?;

  Ok(())
}
