use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::{ChatKind, MessageKind};
use utxo_global_tgbot_api::repositories::chatbot::ask_bot;
use utxo_global_tgbot_api::repositories::db::{migrate_db, DB_POOL};
use utxo_global_tgbot_api::{config, repositories};

#[tokio::main]
async fn main() {
    env_logger::init();
    let db = &DB_POOL.clone();
    let member_dao = Arc::new(repositories::member::MemberDao::new(db.clone()));
    let bot_name: Arc<String> = Arc::new(config::get("bot_name"));
    let bot_token: String = config::get("bot_token");

    // migrate db
    if let Err(e) = migrate_db().await {
        println!("\nMigrate db failed: {}", e);
    }

    // Initialize the bot
    let bot = Bot::new(bot_token);
    log::info!("Start TGBot");

    // Create an update handler
    teloxide::repl(bot, {
        let member_dao = Arc::clone(&member_dao);
        let bot_name = Arc::clone(&bot_name);
        move |bot: Bot, message: Message| {
            let member_dao = Arc::clone(&member_dao);
            let bot_name = Arc::clone(&bot_name);
            async move {
                // Handle new chat members
                if let MessageKind::NewChatMembers(msg) = &message.kind {
                    for user in msg.clone().new_chat_members {
                        let tgid = user.id;
                        let tgname = user.clone().username.unwrap_or(user.full_name());
                        let lyc_link: String = config::get("kyc_link");

                        // handle error
                        if bot
                            .send_message(
                                tgid,
                                format!(
                                    "
                    Hello {}, welcome to the group! 👋\n
                    Please complete your KYC to get started.\n
                    Click the link below to begin:\n
                    [KYC Form]({})
                    Please ask me anything if you need any help\n
                    ",
                                    tgname, lyc_link
                                ),
                            )
                            .await
                            .is_ok()
                        {
                            log::info!("Sent KYC message to {tgname} (ID: {tgid})");
                            // Insert new member
                            if let Err(err) = member_dao.insert_member(tgid.0 as i64, tgname).await
                            {
                                log::error!("insert new member failed: {:?}", err);
                            }
                        } else {
                            log::error!("Could not message {tgname} (ID: {tgid})");
                        }
                    }
                }

                if let ChatKind::Private(..) = &message.chat.kind {
                    if let Some(text) = message.clone().text() {
                        if let Some(user) = message.clone().from {
                            let tgid = user.id;

                            let response =
                                ask_bot(text, bot_name.as_str(), tgid.to_string().as_str());
                            if let Err(error) = bot.send_message(message.chat.id, response).await {
                                log::error!("Reply private message: {:?}", error);
                            };
                        }
                    }
                }

                Ok(())
            }
        }
    })
    .await;
}
