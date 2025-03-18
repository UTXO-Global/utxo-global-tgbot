use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use teloxide::{
    dispatching::dialogue::GetChatId, payloads::{BanChatMemberSetters, SendMessageSetters}, prelude::*, types::{Chat, ChatKind, ChatMemberStatus, ChatPermissions, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageKind, ParseMode}, utils::command::BotCommands, Bot
};

use crate::{config::{self, MEMBER_BAN_DURATION, MEMBER_KYC_DURATION}, models::telegram::{TelegramGroup, TelegramGroupAdmin, TelegramGroupJoined, MEMBER_STATUS_PENDING, MEMBER_STATUS_REJECT}, repositories::{member::MemberDao, telegram::TelegramDao}};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum CommandType {
    SetToken(String),
    SetAmount(i64),
    SetAge(i32),
    GroupConfig,
    ListUsers
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum PrivateCommandType {
    MyGroups,
    GroupConfig(String),
    ListUsers(String),
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
                    } else if let ChatKind::Private(..) = chat.clone().kind{
                        if let Ok(command) = PrivateCommandType::parse(message.text().clone().unwrap_or(""), "bot") {
                            service.handle_private_command(&bot, message, command).await;
                        }
                    }
                    respond(())
                }
            }
        })
        .await;
    }

    pub async fn update_group_admin(&self, bot: Bot, chat: Chat) {
        let admins: Vec<teloxide::types::ChatMember> = bot.get_chat_administrators(chat.id.to_string()).send().await.unwrap();
            for user in admins.clone() {
                if user.is_administrator() || user.is_owner() {
                    let _ = self.tele_dao.add_admin(TelegramGroupAdmin{ 
                        chat_id: chat.id.to_string(), 
                        user_id: user.user.id.0 as i64, 
                        created_at: Utc::now().naive_utc(), 
                        updated_at: Utc::now().naive_utc() }).await;
                }
            }
    }

    pub async fn handle_message(&self, bot: &Bot, message: Message) {
        let chat = message.chat.clone();
                
        let text = message.text().clone().unwrap_or("");

        // Handle new chat members
        if let MessageKind::NewChatMembers(msg) = &message.kind.clone(){
            let chat_title = match chat.kind.clone() {
                teloxide::types::ChatKind::Public(chat_public) => &chat_public.title.unwrap_or("".to_string()),
                teloxide::types::ChatKind::Private(chat_private) =>  &chat_private.first_name.unwrap_or("".to_string()),
            }; 
            let _ = self.update_group_admin(bot.clone(), chat.clone()).await;

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
                            "Visit",
                            reqwest::Url::from_str(kyc_link.as_str()).unwrap(),
                        )]]);
                // handle error
                if let Err(err) = bot
                    .send_message(
                        message.chat_id().unwrap(),
                        format!(
                            "Hello @{tgname}, welcome to the group! 👋\nPlease complete your information to get started.\n"

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
                    let expired = Utc::now().naive_utc() + MEMBER_KYC_DURATION;
                    if member_joined.is_none() {
                        let _ = self.tele_dao.add_member(TelegramGroupJoined{
                            chat_id: chat.id.to_string(), 
                            user_id: tgid.0 as i64,
                            user_name: tgname.clone(), 
                            ckb_address: None,
                            dob: None,
                            status: 0, 
                            expired,
                            created_at: Utc::now().naive_utc(), 
                            updated_at: Utc::now().naive_utc() 
                        }).await;
                    } else {
                        let _ = self.tele_dao.update_member(None, None, chat.id.to_string(), tgid.0 as i64, expired, MEMBER_STATUS_PENDING).await;
                    }
                }
            }
        } else if text.starts_with("/"){
            if let Ok(command) = CommandType::parse(text, "bot") {
                self.handle_command(bot, message.clone(), command).await;
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
                        let _ = bot.delete_message(chat.id, message.id).await;
                    }
                }
                CommandType::SetAmount(amount) => {
                    if is_admin {
                        group.min_approve_balance = Some(amount);
                        let _ = self.tele_dao.update_group(&group).await;
                    } else {
                        let _ = bot.delete_message(chat.id, message.id).await;
                    }
                }
                CommandType::SetAge(age) => {
                    if is_admin {
                        group.min_approve_age = Some(age);
                        let _ = self.tele_dao.update_group(&group).await;
                    } else {
                        let _ = bot.delete_message(chat.id, message.id).await;
                    }
                }
                CommandType::GroupConfig => {
                    if is_admin {
                        self.send_group_config_to_admin(bot.clone(), group.chat_id, chat).await;
                    }
                },
                CommandType::ListUsers => {
                    if is_admin {
                        self.send_list_users_to_admin(bot.clone(), group.chat_id, chat).await;
                    }
                },
            }
        }
    }

    pub async fn handle_private_command(&self, bot: &Bot, message: Message, command: PrivateCommandType) {
        let chat = message.chat.clone();
        if let Some(user) = message.from {
            match command {
                PrivateCommandType::MyGroups => {
                    let groups: Vec<TelegramGroup> = self.tele_dao.get_group_by_admin(user.id.0 as i64).await.unwrap_or(vec![]);
                    let mut table = String::from("<pre>\n");
                    table.push_str("+-----------------+----------------------+\n");
                    table.push_str("| GroupId         | Name                 |\n");
                    table.push_str("+-----------------+----------------------+\n");
                    for group in groups {
                        table.push_str(&format!("| {:<15} | {:<20} |\n", group.chat_id, group.name))
                    }
                    table.push_str("+-----------------+----------------------+\n");
                    table.push_str("</pre>");
                    bot.send_message(chat.id, table)
                    .parse_mode(ParseMode::Html)
                    .await
                    .unwrap();
                }
                PrivateCommandType::GroupConfig(group_id) => {
                    self.send_group_config_to_admin(bot.clone(), group_id, chat).await;
                }
                PrivateCommandType::ListUsers(group_id) => {
                    self.send_list_users_to_admin(bot.clone(), group_id, chat).await;
                }
            }   
        }
    }

    pub async fn send_group_config_to_admin(&self, bot: Bot, group_id: String, chat: Chat) {
        if let Some(group) = self.tele_dao.get_group(group_id).await.unwrap() {
            let mut table = String::from("<pre>\n");
            table.push_str(&format!("TokenAddress{}\nMin Age: {}\nMin Amount: {}", group.token_address.unwrap_or("".to_string()), group.min_approve_age.unwrap_or(0), group.min_approve_balance.unwrap_or(0)));
            table.push_str("</pre>");
            bot.send_message(chat.id, table)
            .parse_mode(ParseMode::Html)
            .await
            .unwrap();
        }
    }
    pub async fn send_list_users_to_admin(&self, bot: Bot, group_id: String, chat: Chat) {
        let members: Vec<TelegramGroupJoined> = self.tele_dao.get_member_by_group(group_id).await.unwrap_or(vec![]);
        let mut table = String::from("<pre>\n");
        for member in members {
            let dob = if member.dob.is_none() {
                ""
            } else {
                &member.dob.unwrap().to_string()
            };
            table.push_str(&format!("Username: {}\nAddress: {}\nDob: {}\nStatus: {}\n\n", member.user_name, member.ckb_address.unwrap_or("".to_string()), dob, member.status));
        }
        table.push_str("</pre>");
        bot.send_message(chat.id, table)
        .parse_mode(ParseMode::Html)
        .await
        .unwrap();
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

    pub async fn cron_auto_kick_member(&self) {
        if let Ok(members) = self.tele_dao.get_member_not_kyc().await {
            let until_date = Utc::now() + MEMBER_BAN_DURATION;
            for member in members {
                let _ = self.bot
                        .ban_chat_member(
                            member.clone().chat_id.to_string(),
                            UserId(member.clone().user_id as u64),
                        ).until_date(until_date)
                        .await;

                    let _ = self.bot
                        .send_message(
                            member.clone().chat_id.to_string(),
                            format!(
                                "⚠️ User {} banned! \nReason: did not complete verification within 3 minutes",
                                member.clone().user_name,
                            ),
                        )
                        .await;

                    let _ = self
                        .tele_dao
                        .update_member(None, None, member.chat_id, member.user_id, member.expired, MEMBER_STATUS_REJECT)
                        .await;
            }
        }
    }
}
