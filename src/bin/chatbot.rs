use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use teloxide::dispatching::dialogue::GetChatId;
use teloxide::prelude::*;
use teloxide::types::{
    ChatPermissions, InlineKeyboardButton, InlineKeyboardMarkup, MessageKind, ParseMode
};
use utxo_global_tgbot_api::models::telegram::{TelegramGroup, TelegramGroupJoined, MEMBER_STATUS_PENDING};
use utxo_global_tgbot_api::repositories::db::{migrate_db, DB_POOL};
use utxo_global_tgbot_api::{config, repositories};

#[tokio::main]
async fn main() {
    env_logger::init();
    let db = &DB_POOL.clone();
    let member_dao = Arc::new(repositories::member::MemberDao::new(db.clone()));
    let telegram_dao: Arc<repositories::telegram::TelegramDao> = Arc::new(repositories::telegram::TelegramDao::new(db.clone()));
    // let bot_name: Arc<String> = Arc::new(config::get("bot_name"));thấ
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
        move |bot: Bot, message: Message| {
            let telegram_dao = Arc::clone(&telegram_dao);
            let member_dao = Arc::clone(&member_dao);
            async move {
                let chat = message.chat.clone();
                
                if chat.is_group() || chat.is_supergroup(){
                    let chat_title = match chat.kind {
                        teloxide::types::ChatKind::Public(chat_public) => &chat_public.title.unwrap_or("".to_string()),
                        teloxide::types::ChatKind::Private(chat_private) =>  &chat_private.first_name.unwrap_or("".to_string()),
                    }; 
                    let _ = telegram_dao.add_group(TelegramGroup{ 
                        chat_id: chat.id.to_string(), 
                        name: chat_title.to_string(), 
                        status: 1, 
                        token_address: None, 
                        min_approve_balance: Some(0), 
                        min_approve_age: Some(18), 
                        created_at: Utc::now().naive_utc(), 
                        updated_at: Utc::now().naive_utc() }).await;
    
                    // Handle new chat members
                    if let MessageKind::NewChatMembers(msg) = &message.clone().kind {
                        for user in msg.clone().new_chat_members {
                            if user.is_bot{
                                continue
                            }
                            
                            let tgid = user.id;
                            let permissions = ChatPermissions::empty();
                            let _ = bot.restrict_chat_member(chat.id, tgid, permissions).await;
                            
                            let tgname: String = user.clone().username.unwrap_or(user.full_name());
                            let kyc_link: String = config::get("kyc_link");
                            let keyboard =
                                    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::url(
                                        "KYC Form",
                                        reqwest::Url::from_str(kyc_link.as_str()).unwrap(),
                                    )]]);
                            // handle error
                            if let Err(err) = bot
                                .send_message(
                                    message.chat_id().unwrap(),
                                    format!(
                                        "Hello @{tgname}, welcome to the group! 👋\nPlease complete your KYC to get started.\n"
                                    ),
                                ).parse_mode(ParseMode::Html)
                                .reply_markup(keyboard)
                                .await
                            {
                                log::error!(
                                    "Could not message {tgname} (ID: {tgid}). Error: {:?}",
                                    err
                                );
                            } else {
                                if let Err(err) = member_dao.insert_member(tgid.0 as i64, tgname.clone()).await
                                {
                                    log::error!("insert new member failed: {:?}", err);
                                }
    
                                let member_joined =telegram_dao.get_member(chat.id.to_string(), tgid.0 as i64).await.unwrap();
                                if member_joined.is_none() {
                                    let _ = telegram_dao.add_member(TelegramGroupJoined{
                                        chat_id: chat.id.to_string(), 
                                        user_id: tgid.0 as i64,
                                        user_name: tgname.clone(), 
                                        status: 0, 
                                        created_at: Utc::now().naive_utc(), 
                                        updated_at: Utc::now().naive_utc() 
                                    }).await;
                                } else {
                                    let _ = telegram_dao.update_mmember(chat.id.to_string(), tgid.0 as i64, MEMBER_STATUS_PENDING).await;
                                }
                            }
                        }
                    }
    
                    /*if let ChatKind::Private(..) = &message.chat.kind {
                        if let Some(text) = message.clone().text() {
                            // Handle /start command
                            if text == "/start" {
                                let kyc_link: String = config::get("kyc_link");
        
                                let keyboard =
                                    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::url(
                                        "KYC Form",
                                        reqwest::Url::from_str(kyc_link.as_str()).unwrap(),
                                    )]]);
                                bot.send_message(
                                    message.chat.id,
                                    "Welcome 👋!\nPlease complete your KYC to get started.\nPlease ask me anything if you need any help\n",
                                )
                                .reply_markup(keyboard)
                                .await?;
                            } else {
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
                    }*/
    
                }
                Ok(())        
            }
        }
    })
    .await;
}
