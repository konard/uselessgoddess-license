use std::{path::Path, sync::Arc, time::Duration};

use futures::future;
use teloxide::{
  prelude::*,
  types::InputFile,
  utils::command::{BotCommands, ParseError},
};

use super::ReplyBot;
use crate::{
  entity::{license::LicenseType, user::UserRole},
  prelude::*,
  state::{AppState, Services},
  sv::referral::CENTS_PER_DOLLAR,
};

fn parse_publish(
  input: String,
) -> std::result::Result<(String, String, String), ParseError> {
  let mut parts = input.splitn(3, ' ');
  let filename = parts.next().unwrap_or_default().to_string();
  let version = parts.next().unwrap_or_default().to_string();
  let changelog = parts.next().unwrap_or_default().to_string();

  if filename.is_empty() || version.is_empty() {
    return Err(ParseError::IncorrectFormat(
      "Usage: /publish <filename> <version> [changelog]".into(),
    ));
  }

  Ok((filename, version, changelog))
}

fn parse_buy(
  input: String,
) -> std::result::Result<(String, Duration), ParseError> {
  let mut parts = input.splitn(2, ' ');
  let key = parts.next().unwrap_or_default().to_string();
  let duration_str = parts.next().unwrap_or_default().to_string();

  if key.is_empty() || duration_str.is_empty() {
    return Err(ParseError::IncorrectFormat(
      "Usage: /buy <key> <duration>\nExamples: /buy abc123 30d, /buy abc123 2w, /buy abc123 1h30m"
        .into(),
    ));
  }

  let duration = humantime::parse_duration(&duration_str).map_err(|e| {
    ParseError::IncorrectFormat(
      format!(
        "Invalid duration '{}': {}\nExamples: 30d, 2w, 1h30m, 7days",
        duration_str, e
      )
      .into(),
    )
  })?;

  Ok((key, duration))
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
  Start,
  Help,
  Users,
  Gen(String),
  #[command(parse_with = parse_buy)]
  Buy {
    key: String,
    duration: Duration,
  },
  Ban(String),
  Unban(String),
  Info(String),
  Stats,
  Backup,
  Builds,
  #[command(parse_with = parse_publish)]
  Publish {
    filename: String,
    version: String,
    changelog: String,
  },
  Yank(String),
  Unyank(String),
  #[command(hide)]
  Deactivate(String),
  GlobalStats,
  SetRole(String),
  CreateRef(String),
  Refs,
  Deposit(String),
  Withdraw(String),
  RefStats,
}

const ADMIN_HELP: &str = "\
<b>📋 Admin Commands</b>

<b>License Management:</b>
/gen &lt;user_id&gt; [days] - Generate new license
/buy &lt;key&gt; &lt;duration&gt; - Extend license (e.g. 30d, 2w, 1h30m)
/ban &lt;key&gt; - Block license and drop sessions
/unban &lt;key&gt; - Unblock license
/info &lt;key|user_id&gt; - Show license or user details

<b>Build Management:</b>
/builds - List all builds
/publish &lt;file&gt; &lt;ver&gt; [log] - Publish new build
/yank &lt;version&gt; - Remove build from downloads
/unyank &lt;version&gt; - Reactivate yanked build

<b>Referral System:</b>
/setrole &lt;user_id&gt; &lt;role&gt; - Set user role (user/creator/admin)
/createref &lt;code&gt; [rate] [days] - Create referral code
/refs - List all referral codes
/refstats - Show referral statistics

<b>Balance Management:</b>
/deposit &lt;user_id&gt; &lt;amount&gt; - Add balance (in cents)
/withdraw &lt;user_id&gt; &lt;amount&gt; - Process withdrawal

<b>System:</b>
/users - List all registered users
/stats - Show active sessions count
/globalstats - Show global XP/drops summary
/backup - Manual database backup
/help - Show this message";

pub async fn handle(
  app: Arc<AppState>,
  bot: ReplyBot,
  cmd: Command,
) -> ResponseResult<()> {
  let sv = app.sv();

  let _ = sv.user.get_or_create(bot.user_id).await;

  match &cmd {
    Command::Start => {
      let text = "<b>Yet Another Counter Strike Panel!</b>\n\n\
        Use the buttons below to navigate.\n\
        Read docs: https://yacsp.gitbook.io/yacsp\n\
        Contact support: @y_a_c_s_p";
      bot
        .reply_with_keyboard(
          text,
          super::callback::main_menu(sv.license.is_promo_active()),
        )
        .await?;
    }
    Command::Help if app.admins.contains(&bot.user_id) => {
      bot.reply_html(ADMIN_HELP).await?;
      return Ok(());
    }
    Command::Help => {
      bot
        .reply_html("Use /start to access the main menu with buttons.")
        .await?;
      return Ok(());
    }
    _ => {}
  }

  if app.admins.contains(&bot.user_id) {
    handle_admin_command(app, bot, cmd).await?;
  }

  Ok(())
}

async fn process_info_command(
  sv: &Services<'_>,
  app: &AppState,
  bot: &ReplyBot,
  input: String,
) -> Result<String> {
  let input = input.trim();
  if input.is_empty() {
    return Err(Error::InvalidArgs(
      "Usage: /info <license_key | user_id>".into(),
    ));
  }

  if let Ok(user_id) = input.parse::<i64>() {
    let user = sv.user.by_id(user_id).await?.ok_or(Error::UserNotFound)?;
    let username = bot.infer_username(ChatId(user_id)).await;
    let stats = sv.stats.display_stats(user_id).await?;
    let licenses = sv.license.by_user(user_id, true).await?;

    let mut total_active_sessions = 0;
    let mut lic_text = String::new();

    for lic in &licenses {
      let active = app.sessions.get(&lic.key).map(|s| s.len()).unwrap_or(0);
      total_active_sessions += active;

      let status_icon = if lic.is_blocked {
        "⛔"
      } else if lic.expires_at < Utc::now().naive_utc() {
        "❌"
      } else if active > 0 {
        "🟢"
      } else {
        "⚪"
      };

      lic_text.push_str(&format!(
        "{} <code>{}</code> ({:?})\n",
        status_icon, lic.key, lic.license_type
      ));
    }

    let role_str = match user.role {
      UserRole::User => "User",
      UserRole::Creator => "Creator",
      UserRole::Admin => "Admin",
    };

    let balance_str =
      format!("${:.2}", user.balance as f64 / CENTS_PER_DOLLAR as f64);
    let referral_str = user.referred_by.as_deref().unwrap_or("None");

    return Ok(format!(
      "👤 <b>User Info</b>\n\
      ID: <code>{}</code>\n\
      Name: {}\n\
      Registered: {}\n\
      Role: {}\n\
      Balance: {}\n\
      Referred by: {}\n\n\
      📊 <b>Global Stats</b>\n\
      XP (Week/Total): {} / {}\n\
      Runtime: {:.1}h\n\
      Total Sessions: {}\n\n\
      🔑 <b>Licenses ({})</b>\n\
      {}",
      user.tg_user_id,
      username,
      utils::format_date(user.reg_date),
      role_str,
      balance_str,
      referral_str,
      stats.weekly_xp,
      stats.total_xp,
      stats.runtime_hours,
      total_active_sessions,
      licenses.len(),
      if lic_text.is_empty() { "No licenses" } else { &lic_text }
    ));
  }

  let key = input;
  let license = sv.license.by_key(key).await?.ok_or(Error::LicenseNotFound)?;
  let username = bot.infer_username(ChatId(license.tg_user_id)).await;

  let sessions = app.sessions.get(key);
  let active_count = sessions.as_ref().map(|s| s.len()).unwrap_or(0);
  let now = Utc::now().naive_utc();

  let status = if license.is_blocked {
    "⛔ BLOCKED"
  } else if license.expires_at < now {
    "❌ EXPIRED"
  } else if active_count > 0 {
    "🟢 ONLINE"
  } else {
    "⚪ OFFLINE"
  };

  let duration_left = if license.expires_at > now {
    utils::format_duration(license.expires_at - now)
  } else {
    "0d 0h".to_string()
  };

  let mut text = format!(
    "🔑 <b>License Info</b>\n\n\
    <b>Key:</b> <code>{}</code>\n\
    <b>Type:</b> {:?}\n\
    <b>Status:</b> {}\n\
    <b>Owner:</b> {} (<code>{}</code>)\n\n\
    📅 <b>Timeline</b>\n\
    Created: {}\n\
    Expires: {} (in {})\n\n\
    🖥 <b>Sessions ({}/{})</b>\n",
    license.key,
    license.license_type,
    status,
    username,
    license.tg_user_id,
    utils::format_date(license.created_at),
    utils::format_date(license.expires_at),
    duration_left,
    active_count,
    license.max_sessions
  );

  if let Some(sess_list) = sessions {
    for (i, s) in sess_list.iter().enumerate() {
      text.push_str(&format!(
        " {}. ID: <code>{}...</code>\n    HWID: <code>{}</code>\n",
        i + 1,
        &s.session_id.chars().take(8).collect::<String>(),
        s.hwid_hash.as_deref().unwrap_or("Unknown")
      ));
    }
  } else if active_count == 0 {
    text.push_str(" <i>No active sessions</i>");
  }

  Ok(text)
}

async fn handle_admin_command(
  app: Arc<AppState>,
  bot: ReplyBot,
  cmd: Command,
) -> ResponseResult<()> {
  let sv = app.sv();

  if let Command::Users = cmd {
    let users_data = match sv.user.all_with_licenses().await {
      Ok(u) => u,
      Err(e) => {
        bot.reply_html(format!("❌ DB Error: {}", e)).await?;
        return Ok(());
      }
    };

    if users_data.is_empty() {
      bot.reply_html("📭 Database is empty.").await?;
      return Ok(());
    }

    bot
      .reply_html(format!("⏳ Loading data for {} users...", users_data.len()))
      .await?;

    let user_futures = users_data.into_iter().map(|(u, licenses)| {
      let bot = bot.clone();
      async move {
        let username = bot.infer_username(ChatId(u.tg_user_id)).await;
        (u, username, licenses)
      }
    });

    let resolved_users = future::join_all(user_futures).await;

    let mut text =
      format!("👥 <b>Users List (Total: {})</b>\n\n", resolved_users.len());
    let now = Utc::now().naive_utc();

    for (i, (user, username, licenses)) in resolved_users.iter().enumerate() {
      let status_icon = if licenses.is_empty() {
        "📂"
      } else {
        let mut has_online = false;
        let mut has_valid = false;
        let mut has_blocked = false;

        for lic in licenses {
          if lic.is_blocked {
            has_blocked = true;
            continue;
          }

          if lic.expires_at > now {
            has_valid = true;
            if let Some(sessions) = app.sessions.get(&lic.key)
              && !sessions.is_empty()
            {
              has_online = true;
              break;
            }
          }
        }

        if has_online {
          "🟢"
        } else if has_valid {
          "⚪"
        } else if has_blocked {
          "⛔"
        } else {
          "❌"
        }
      };

      text.push_str(&format!(
        "<b>{}.</b> {} {} <code>{}</code>\n",
        i + 1,
        status_icon,
        username,
        user.tg_user_id
      ));
    }

    // Use chunked reply to handle long user lists
    bot.reply_html_chunked(text).await?;
    return Ok(());
  }

  let result: Result<String> = match cmd {
    Command::Gen(args) => {
      let parts: Vec<&str> = args.split_whitespace().collect();
      let (target_user, days) = match parts.as_slice() {
        [user_id] => (user_id.parse::<i64>().ok(), 0u64),
        [user_id, days] => {
          (user_id.parse::<i64>().ok(), days.parse::<u64>().unwrap_or(0))
        }
        _ => (None, 0),
      };

      match target_user {
        Some(target_user) => sv
          .license
          .create(target_user, LicenseType::Pro, days)
          .await
          .map(|l| format!("✅ Key created:\n<code>{}</code>", l.key)),
        None => Err(Error::InvalidArgs("Usage: /gen <user_id> [days]".into())),
      }
    }

    Command::Buy { key, duration } => {
      sv.license.expires(&key, duration).await.map(|new_exp| {
        let duration_str = humantime::format_duration(duration);
        format!(
          "✅ Key expires after {duration_str}.\nNew expiry: <code>{}</code>",
          utils::format_date(new_exp)
        )
      })
    }

    Command::Ban(key) => {
      let result = sv.license.set_blocked(&key, true).await;
      if result.is_ok() {
        app.drop_sessions(&key);
      }
      result.map(|_| "🚫 Key blocked, sessions dropped".into())
    }

    Command::Unban(key) => sv
      .license
      .set_blocked(&key, false)
      .await
      .map(|_| "✅ Key unblocked".into()),

    Command::Info(input) => process_info_command(&sv, &app, &bot, input).await,
    Command::Backup => {
      if app.perform_backup(bot.chat_id).await.is_err() {
        bot.send_document(InputFile::file("licenses.db")).await?;
      }
      return Ok(());
    }
    Command::Builds => match sv.build.all().await {
      Ok(builds) if !builds.is_empty() => {
        let mut text = String::from("<b>All Builds:</b>\n");
        for build in builds {
          let status = if build.is_active { "✅" } else { "❌" };
          text.push_str(&format!(
            "\n{} <b>v{}</b>\n{} downloads\n{}\n",
            status,
            build.version,
            build.downloads,
            utils::format_date(build.created_at)
          ));
          if let Some(changelog) = &build.changelog {
            text.push_str(&format!("<code>{}</code>\n", changelog));
          }
        }
        bot.reply_html(text).await?;
        return Ok(());
      }
      Ok(_) => Err(Error::BuildNotFound),
      Err(e) => Err(e),
    },

    Command::Publish { filename, version, changelog } => {
      let file_path = format!("{}/{}", app.config.builds_directory, filename);
      let path = Path::new(&file_path);

      if !path.exists() {
        Err(Error::InvalidArgs(format!(
          "File not found: {}\n\nUpload the file to the builds folder using scp:\nscp file.exe server:{}/",
          file_path, app.config.builds_directory
        )))
      } else {
        let changelog_opt =
          if changelog.is_empty() { None } else { Some(changelog) };

        sv.build.create(version.clone(), file_path, changelog_opt).await.map(
          |build| {
            format!(
              "✅ Build published!\n\n\
              <b>Version:</b> {}\n\
              <b>File:</b> {}\n\
              <b>Created:</b> {}",
              build.version,
              build.file_path,
              utils::format_date(build.created_at)
            )
          },
        )
      }
    }

    Command::Yank(version) | Command::Deactivate(version) => {
      async {
        let build =
          sv.build.by_version(&version).await?.ok_or(Error::BuildNotFound)?;
        if !build.is_active {
          return Err(Error::BuildInactive);
        }
        sv.build.deactivate(&version).await?;
        Ok(format!(
          "✅ Build yanked (removed from downloads).\n\n\
        <b>Version:</b> {}\n\
        <b>Downloads:</b> {}",
          build.version, build.downloads
        ))
      }
      .await
    }

    Command::Unyank(version) => {
      async {
        let build =
          sv.build.by_version(&version).await?.ok_or(Error::BuildNotFound)?;
        if build.is_active {
          return Err(Error::BuildAlreadyActive);
        }
        sv.build.activate(&version).await?;
        Ok(format!(
          "✅ Build reactivated (available for downloads).\n\n\
        <b>Version:</b> {}\n\
        <b>Downloads:</b> {}",
          build.version, build.downloads
        ))
      }
      .await
    }

    Command::GlobalStats => {
      async {
        let stats = sv.stats.aggregate().await?;
        Ok(format!(
          "📊 <b>Global Stats</b>\n\n\
          <b>XP:</b>\n\
          Weekly: {}\n\
          Total: {}\n\n\
          <b>Drops:</b> {}\n\
          <b>Runtime:</b> {:.1}h\n\
          <b>Active instances:</b> {}",
          stats.weekly_xp,
          stats.total_xp,
          stats.total_drops,
          stats.total_runtime_hours,
          stats.active_instances
        ))
      }
      .await
    }

    Command::Stats => Ok(format!(
      "Active Keys: {}\n\
       Active Sessions: {}",
      app.sessions.iter().map(|kv| kv.value().len()).sum::<usize>(),
      app.sessions.len()
    )),

    Command::SetRole(args) => {
      async {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
          [user_id_str, role_str] => {
            let user_id = user_id_str
              .parse::<i64>()
              .map_err(|_| Error::InvalidArgs("Invalid user ID".into()))?;
            let role = match *role_str {
              "user" => UserRole::User,
              "creator" => UserRole::Creator,
              "admin" => UserRole::Admin,
              _ => {
                return Err(Error::InvalidArgs(
                  "Invalid role. Use: user, creator, admin".into(),
                ));
              }
            };
            sv.user.set_role(user_id, role.clone()).await?;
            Ok(format!("✅ User {} role set to {:?}", user_id, role))
          }
          _ => {
            Err(Error::InvalidArgs("Usage: /setrole <user_id> <role>".into()))
          }
        }
      }
      .await
    }

    Command::CreateRef(args) => {
      async {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let (code, rate, bonus_days) = match parts.as_slice() {
          [code] => (code.to_string(), 25, 5),
          [code, rate_str] => {
            let rate = rate_str.parse::<i32>().unwrap_or(25);
            (code.to_string(), rate, 5)
          }
          [code, rate_str, days_str] => {
            let rate = rate_str.parse::<i32>().unwrap_or(25);
            let days = days_str.parse::<i32>().unwrap_or(5);
            (code.to_string(), rate, days)
          }
          _ => {
            return Err(Error::InvalidArgs(
              "Usage: /createref <code> [rate%] [bonus_days]".into(),
            ));
          }
        };
        sv.referral
          .create_code(bot.user_id, code.clone(), rate, bonus_days)
          .await?;
        Ok(format!(
          "✅ Referral code created!\n\
          <b>Code:</b> <code>{}</code>\n\
          <b>Commission:</b> {}%\n\
          <b>Bonus days:</b> {}",
          code, rate, bonus_days
        ))
      }
      .await
    }

    Command::Refs => {
      async {
        let refs = sv.referral.all().await?;
        if refs.is_empty() {
          Ok("📭 No referral codes found.".into())
        } else {
          let mut text = String::from("<b>📋 Referral Codes</b>\n\n");
          for r in refs {
            let status = if r.is_active { "✅" } else { "❌" };
            let earnings = r.total_earnings as f64 / CENTS_PER_DOLLAR as f64;
            text.push_str(&format!(
              "{} <code>{}</code>\n\
              Owner: <code>{}</code>\n\
              Rate: {}% | Bonus: {}d\n\
              Sales: {} | Earned: ${:.2}\n\n",
              status,
              r.code,
              r.owner_id,
              r.commission_rate,
              r.bonus_days,
              r.total_sales,
              earnings
            ));
          }
          Ok(text)
        }
      }
      .await
    }

    Command::RefStats => {
      async {
        let refs = sv.referral.all().await?;
        let total_sales: i32 = refs.iter().map(|r| r.total_sales).sum();
        let total_earnings: i64 = refs.iter().map(|r| r.total_earnings).sum();
        let active_count = refs.iter().filter(|r| r.is_active).count();

        Ok(format!(
          "📊 <b>Referral Statistics</b>\n\n\
          <b>Total codes:</b> {} ({} active)\n\
          <b>Total sales:</b> {}\n\
          <b>Total commissions:</b> ${:.2}",
          refs.len(),
          active_count,
          total_sales,
          total_earnings as f64 / CENTS_PER_DOLLAR as f64
        ))
      }
      .await
    }

    Command::Deposit(args) => {
      async {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
          [user_id_str, amount_str] => {
            let user_id = user_id_str
              .parse::<i64>()
              .map_err(|_| Error::InvalidArgs("Invalid user ID".into()))?;
            let amount = amount_str
              .parse::<i64>()
              .map_err(|_| Error::InvalidArgs("Invalid amount".into()))?;
            let new_balance = sv
              .balance
              .deposit(user_id, amount, Some("Admin deposit".into()))
              .await?;
            Ok(format!(
              "✅ Deposited {} cents to user {}\n\
              New balance: ${:.2}",
              amount,
              user_id,
              new_balance as f64 / CENTS_PER_DOLLAR as f64
            ))
          }
          _ => Err(Error::InvalidArgs(
            "Usage: /deposit <user_id> <amount_cents>".into(),
          )),
        }
      }
      .await
    }

    Command::Withdraw(args) => {
      async {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
          [user_id_str, amount_str] => {
            let user_id = user_id_str
              .parse::<i64>()
              .map_err(|_| Error::InvalidArgs("Invalid user ID".into()))?;
            let amount = amount_str
              .parse::<i64>()
              .map_err(|_| Error::InvalidArgs("Invalid amount".into()))?;
            let new_balance = sv.balance.withdraw(user_id, amount).await?;
            Ok(format!(
              "✅ Withdrawal of {} cents processed for user {}\n\
              New balance: ${:.2}",
              amount,
              user_id,
              new_balance as f64 / CENTS_PER_DOLLAR as f64
            ))
          }
          _ => Err(Error::InvalidArgs(
            "Usage: /withdraw <user_id> <amount_cents>".into(),
          )),
        }
      }
      .await
    }

    _ => return Ok(()),
  };

  match result {
    Ok(text) => {
      bot.reply_html(text).await?;
    }
    Err(e) => {
      bot.reply_html(format!("❌ {}", e.user_message())).await?;
    }
  }

  Ok(())
}
