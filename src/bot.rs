use chrono::{Duration, Utc};
use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};
use teloxide::utils::command::BotCommands;
use uuid::Uuid;

use crate::state::App;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "help")]
    Help,
    #[command(description = "gen <days> <user_id>", parse_with = "split")]
    Gen(i64, i64),
    #[command(description = "backup")]
    Backup,
}

async fn update(app: App, bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    if !app.admins.contains(&msg.chat.id.0) {
        return Ok(());
    }

    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
        }
        Command::Gen(days, user_id) => {
            let key = Uuid::new_v4().to_string();
            let exp = (Utc::now() + Duration::days(days)).naive_utc();

            let insert = sqlx::query!(
                "INSERT INTO licenses (key, tg_user_id, expires_at) VALUES (?, ?, ?)",
                key,
                user_id,
                exp
            )
            .execute(&app.db)
            .await;

            if let Err(err) = insert {
                bot.send_message(msg.chat.id, format!("Error: {err:?}")).await?
            } else {
                bot.send_message(msg.chat.id, format!("Key: <code>{}</code>", key))
                    .parse_mode(ParseMode::Html)
                    .await?
            };
        }
        Command::Backup => {
            if let Err(err) = app.perform_backup(msg.chat.id).await {
                let _ = bot.send_document(msg.chat.id, InputFile::file("licenses.db")).await;
                bot.send_message(msg.chat.id, format!("Backup failed: {err:?}")).await?;
            }
        }
    };
    respond(())
}

pub async fn run_bot(app: App) {
    let bot = app.bot.clone();
    let handler = Update::filter_message().filter_command::<Command>().endpoint(
        move |bot: Bot, msg: Message, cmd: Command| {
            let app = app.clone();
            update(app, bot, msg, cmd)
        },
    );

    Dispatcher::builder(bot, handler).enable_ctrlc_handler().build().dispatch().await;
}
