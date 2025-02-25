use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use teloxide::{
    dispatching::dialogue::GetChatId, payloads::SendMessageSetters, prelude::*, types::{Chat, ChatMemberStatus, ChatPermissions, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageKind, ParseMode}, utils::command::BotCommands, Bot
};

use crate::{config, models::telegram::{TelegramGroup, TelegramGroupJoined, MEMBER_STATUS_PENDING}, repositories::{member::MemberDao, telegram::TelegramDao}};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum CommandType {
    SetToken(String),
    SetAmount(i64),
    SetAge(i32),
}

#[derive(Clone, Debug)]
pub struct TelegramService {
    pub member_dao: Arc<MemberDao>,
    pub tele_dao: Arc<TelegramDao>,
    pub bot: Bot,
}

impl TelegramService {
    pub fn new(member_dao: Arc<MemberDao>, tele_dao: Arc<TelegramDao>) -> Self {
        let bot_token: String = config::get("bot_token");
        TelegramService {
            member_dao: member_dao.clone(),
            tele_dao: tele_dao.clone(),
            bot: Bot::new(bot_token)
        }
    }

    pub async fn start(self: Arc<Self>){
        println!("Telegram Bot Runnig....");
        teloxide::repl(self.bot.clone(), {
            move |bot: Bot, message: Message| {
                let service: Arc<TelegramService> = Arc::clone(&self);
                async move {
                    let chat = message.chat.clone();
                    if chat.is_group() || chat.is_supergroup() {
                        service.handle_message(&bot, message).await;
                    }
                    respond(())
                }
            }
        })
        .await;
    }

    pub async fn handle_message(&self, bot: &Bot, message: Message) {
        let chat = message.chat.clone();
                
        let text = message.text().unwrap_or("");
        println!("{:?} {:?}", text, message.kind.clone());
        // Handle new chat members
        
        if let MessageKind::NewChatMembers(msg) = &message.kind.clone(){
            println!("NewChatMembers here!");
            let chat_title = match chat.kind {
                teloxide::types::ChatKind::Public(chat_public) => &chat_public.title.unwrap_or("".to_string()),
                teloxide::types::ChatKind::Private(chat_private) =>  &chat_private.first_name.unwrap_or("".to_string()),
            }; 
            let _ = self.tele_dao.add_group(TelegramGroup{ 
                chat_id: chat.id.to_string(), 
                name: chat_title.to_string(), 
                status: 1, 
                token_address: None, 
                min_approve_balance: Some(0), 
                min_approve_age: Some(18), 
                created_at: Utc::now().naive_utc(), 
                updated_at: Utc::now().naive_utc() }).await;

            
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
                    if let Err(err) = self.member_dao.insert_member(tgid.0 as i64, tgname.clone()).await
                    {
                        log::error!("insert new member failed: {:?}", err);
                    }

                    let member_joined =self.tele_dao.get_member(chat.id.to_string(), tgid.0 as i64).await.unwrap();
                    if member_joined.is_none() {
                        let _ = self.tele_dao.add_member(TelegramGroupJoined{
                            chat_id: chat.id.to_string(), 
                            user_id: tgid.0 as i64,
                            user_name: tgname.clone(), 
                            status: 0, 
                            created_at: Utc::now().naive_utc(), 
                            updated_at: Utc::now().naive_utc() 
                        }).await;
                    } else {
                        let _ = self.tele_dao.update_mmember(chat.id.to_string(), tgid.0 as i64, MEMBER_STATUS_PENDING).await;
                    }
                }
            }
        } else if text.starts_with("/"){
            if let Ok(command) = CommandType::parse(text, "bot") {
                self.handle_command(bot, message, command).await;
            }
        }
    }

    pub async fn handle_command(&self, bot: &Bot, message: Message, command: CommandType) {
        let is_admin = self.is_admin(message.clone(), bot).await;
        let chat = message.chat.clone();
        if let Some(mut group) = self.get_group_or_create(chat.clone()).await {
            match command {
                CommandType::SetToken(token) => {
                    if is_admin {
                        group.token_address = Some(token);
                        let _ = self.tele_dao.update_group(&group).await;
                    } else {
                        let _ = bot.delete_message(chat.id.clone(), message.id.clone()).await;
                    }
                }
                CommandType::SetAmount(amount) => {
                    if is_admin {
                        group.min_approve_balance = Some(amount);
                        let _ = self.tele_dao.update_group(&group).await;
                    } else {
                        let _ = bot.delete_message(chat.id.clone(), message.id.clone()).await;
                    }
                }
                CommandType::SetAge(age) => {
                    if is_admin {
                        group.min_approve_age = Some(age);
                        let _ = self.tele_dao.update_group(&group).await;
                    } else {
                        let _ = bot.delete_message(chat.id.clone(), message.id.clone()).await;
                    }
                }
            }
        }
    }

    pub async fn get_group_or_create(&self, chat: Chat) -> Option<TelegramGroup> {
        let group = self.tele_dao.get_group(chat.id.to_string()).await.unwrap();
        if group.is_none() {
            if let Ok(group) = self.tele_dao.add_group(TelegramGroup{ 
                chat_id: chat.id.to_string(), 
                name: chat.title().unwrap().to_string(), 
                status: 1, 
                token_address: None, 
                min_approve_balance: Some(0), 
                min_approve_age: Some(18), 
                created_at: Utc::now().naive_utc(), 
                updated_at: Utc::now().naive_utc() }).await {
                    return Some(group);
                }
            return None;
        }
        group
    }
    pub async fn is_admin(&self, message: Message, bot: &Bot) -> bool {
        if let Some(user) = message.from {
            match bot.get_chat_member(message.chat.id, user.id).send().await {
                Ok(member) => {
                    return matches!(
                        member.status(),
                        ChatMemberStatus::Administrator | ChatMemberStatus::Owner
                    )
                }
                Err(_) => return false,
            }
        }
        false
    }
}
