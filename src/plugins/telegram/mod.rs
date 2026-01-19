mod callback;
mod command;

use std::{collections::HashSet, sync::Arc};

use command::{AdminCommand, Command, UserCommand};
use teloxide::{
  Bot, RequestError,
  dispatching::{Dispatcher, HandlerExt, UpdateFilterExt},
  prelude::*,
  types::{
    BotCommandScope, CallbackQuery, ChatId, InlineKeyboardMarkup, InputFile,
    Message, MessageId, ParseMode, Update,
  },
  utils::command::BotCommands,
};

use crate::{prelude::*, state::AppState};

pub struct Plugin;

#[async_trait::async_trait]
impl super::Plugin for Plugin {
  async fn start(&self, app: Arc<AppState>) -> anyhow::Result<()> {
    run_bot(app).await;
    Ok(())
  }
}

/// Set up command hints for users and admins.
/// Users see only user commands, admins see both user and admin commands.
async fn setup_commands(bot: &Bot, admins: &HashSet<i64>) {
  // Set user commands as default for all users
  if let Err(e) = bot
    .set_my_commands(UserCommand::bot_commands())
    .scope(BotCommandScope::Default)
    .await
  {
    warn!("Failed to set default commands: {}", e);
  }

  // Set combined commands for each admin (user commands + admin commands)
  let mut admin_commands = UserCommand::bot_commands();
  admin_commands.extend(AdminCommand::bot_commands());

  for &admin_id in admins {
    if let Err(e) = bot
      .set_my_commands(admin_commands.clone())
      .scope(BotCommandScope::Chat { chat_id: ChatId(admin_id).into() })
      .await
    {
      warn!("Failed to set admin commands for {}: {}", admin_id, e);
    }
  }

  info!(
    "Command hints configured: {} user commands, {} admin commands",
    UserCommand::bot_commands().len(),
    AdminCommand::bot_commands().len()
  );
}

pub async fn run_bot(app: Arc<AppState>) {
  info!("Starting Telegram bot...");

  let bot = app.bot.clone();

  // Set up command hints for users and admins
  setup_commands(&bot, &app.admins).await;

  let handler = teloxide::dptree::entry()
    .branch(Update::filter_message().filter_command::<Command>().endpoint({
      let app = app.clone();
      move |bot: Bot, msg: Message, cmd: Command| {
        let app = app.clone();
        let bot = ReplyBot::new(bot, msg.chat.id.0, msg.chat.id, msg.id);
        command::handle(app, bot, cmd)
      }
    }))
    .branch(Update::filter_callback_query().endpoint({
      let app = app.clone();
      move |bot: Bot, query: CallbackQuery| {
        let app = app.clone();
        callback_handle(app, bot, query)
      }
    }));

  Dispatcher::builder(bot, handler).build().dispatch().await;
}

async fn callback_handle(
  app: Arc<AppState>,
  bot: Bot,
  query: CallbackQuery,
) -> ResponseResult<()> {
  if let Some(data) = query.data
    && let Some(msg) = query.message.as_ref()
  {
    let bot =
      ReplyBot::new(bot, query.from.id.0 as i64, msg.chat().id, msg.id());

    // answer callback to remove loading state
    bot.inner.answer_callback_query(query.id.clone()).await?;

    callback::handle(app, bot, &data).await
  } else {
    Ok(())
  }
}

#[derive(Debug, Clone)]
struct ReplyBot {
  inner: Bot,
  pub user_id: i64,
  pub chat_id: ChatId,
  pub message_id: MessageId,
}

impl ReplyBot {
  pub fn new(
    inner: Bot,
    user_id: i64,
    chat_id: ChatId,
    message_id: MessageId,
  ) -> Self {
    Self { inner, user_id, chat_id, message_id }
  }

  async fn reply_html(
    &self,
    text: impl Into<String>,
  ) -> ResponseResult<Message> {
    self
      .inner
      .send_message(self.chat_id, text.into())
      .parse_mode(ParseMode::Html)
      .await
  }

  /// Send a potentially long message by splitting it into chunks if needed.
  /// Returns the last message sent, or error if any chunk fails.
  async fn reply_html_chunked(
    &self,
    text: impl Into<String>,
  ) -> ResponseResult<Message> {
    let chunks = utils::chunk_message(&text.into(), 0);
    let mut last_msg = None;

    for chunk in chunks {
      last_msg = Some(
        self
          .inner
          .send_message(self.chat_id, chunk)
          .parse_mode(ParseMode::Html)
          .await?,
      );
    }

    // chunks is never empty, so last_msg is always Some
    Ok(last_msg.unwrap())
  }

  /// Send a potentially long message by splitting it into chunks.
  /// The keyboard is only attached to the last message.
  /// Returns the last message sent, or error if any chunk fails.
  pub async fn reply_html_chunked_with_keyboard(
    &self,
    text: impl Into<String>,
    keyboard: InlineKeyboardMarkup,
  ) -> ResponseResult<Message> {
    let chunks = utils::chunk_message(&text.into(), 0);
    let total_chunks = chunks.len();
    let mut last_msg = None;

    for (i, chunk) in chunks.into_iter().enumerate() {
      let is_last = i == total_chunks - 1;
      last_msg = Some(if is_last {
        // Attach keyboard only to the last message
        self
          .inner
          .send_message(self.chat_id, chunk)
          .parse_mode(ParseMode::Html)
          .reply_markup(keyboard.clone())
          .await?
      } else {
        self
          .inner
          .send_message(self.chat_id, chunk)
          .parse_mode(ParseMode::Html)
          .await?
      });
    }

    // chunks is never empty, so last_msg is always Some
    Ok(last_msg.unwrap())
  }

  async fn reply_with_keyboard(
    &self,
    text: impl Into<String>,
    keyboard: InlineKeyboardMarkup,
  ) -> ResponseResult<Message> {
    self
      .inner
      .send_message(self.chat_id, text.into())
      .parse_mode(ParseMode::Html)
      .reply_markup(keyboard)
      .await
  }

  pub async fn edit_with_keyboard(
    &self,
    text: impl Into<String>,
    keyboard: InlineKeyboardMarkup,
  ) -> ResponseResult<()> {
    self
      .inner
      .edit_message_text(self.chat_id, self.message_id, text.into())
      .parse_mode(ParseMode::Html)
      .reply_markup(keyboard)
      .await?;
    Ok(())
  }

  async fn send_document(
    &self,
    document: InputFile,
  ) -> Result<Message, RequestError> {
    self.inner.send_document(self.chat_id, document).await
  }

  async fn infer_username(&self, chat_id: ChatId) -> String {
    match self.inner.get_chat(chat_id).await {
      Ok(chat) => {
        if let Some(username) = chat.username() {
          format!("@{}", username)
        } else {
          format!("<a href=\"tg://user?id={}\">unknown</a>", chat_id)
        }
      }
      Err(_) => format!("<code>{}</code> (API Error)", chat_id),
    }
  }
}
