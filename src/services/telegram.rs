use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use serde_json::{json, Value};
use teloxide::{
    dispatching::dialogue::GetChatId, payloads::{BanChatMemberSetters, SendMessageSetters}, prelude::*, types::{Chat, ChatKind, ChatMemberStatus, ChatPermissions, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageKind, ParseMode}, utils::command::BotCommands, Bot
};

use crate::{config::{self, MEMBER_BAN_DURATION, MEMBER_KYC_DURATION}, models::{telegram::{TelegramGroup, TelegramGroupAdmin, TelegramGroupJoined, MEMBER_STATUS_ACCEPTED, MEMBER_STATUS_PENDING, MEMBER_STATUS_REJECT}, token::{Token, TOKEN_TYPE_XUDT}}, repositories::{ckb::get_xudt_info, member::MemberDao, telegram::TelegramDao, token::TokenDao}};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum CommandType {
    SetToken(String),
    SetAmount(i64),
    SetAge(i32),
    GroupConfig,
    ListUsers,
    Sync,
    Help,
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
    pub token_dao: Arc<TokenDao>,
    pub bot: Bot,
}

impl TelegramService {
    pub fn new(member_dao: Arc<MemberDao>, tele_dao: Arc<TelegramDao>, token_dao: Arc<TokenDao>) -> Self {
        let bot_token: String = config::get("bot_token");
        TelegramService {
            member_dao: member_dao.clone(),
            tele_dao: tele_dao.clone(),
            token_dao: token_dao.clone(),
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
                        if let Ok(command) = PrivateCommandType::parse(message.text().unwrap_or(""), "bot") {
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
                
        let text = message.text().unwrap_or("");

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
                            balances: Some("{}".to_owned()),
                            expired,
                            created_at: Utc::now().naive_utc(), 
                            updated_at: Utc::now().naive_utc() 
                        }).await;
                    } else {
                        let _ = self.tele_dao.update_member(None, None, chat.id.to_string(), tgid.0 as i64, expired, MEMBER_STATUS_PENDING, "{}".to_owned()).await;
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
                CommandType::Sync => {
                    if is_admin {
                        // 
                    } else {
                        let _ = bot.delete_message(chat.id, message.id).await;
                    }
                },
                CommandType::SetToken(type_hash) => {
                    if is_admin {
                        group.token_address = Some(type_hash.clone().to_lowercase()); 
                        let _ = self.tele_dao.update_group(&group).await;
                        let _ = self.fetch_token(type_hash).await;
                        
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
                CommandType::Help => {
                    if is_admin {
                        self.send_help_to_admin(bot.clone(), chat).await;
                    } else {
                        let _ = bot.delete_message(chat.id, message.id).await;
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
            let mut token_info: String = "".to_owned();
            if let Some(token) = self.fetch_token(group.token_address.unwrap()).await {
                token_info = format!(
                    "📦 Token Info:\n- Name: {}\n- Symbol: {}\n- Type hash: {}\nScript:{}\n", 
                    token.name.unwrap(),
                    token.symbol.unwrap(),
                    token.type_hash,
                    serde_json::to_string_pretty(&json!({
                        "code_hash": token.code_hash,
                        "hash_type": token.hash_type,
                        "args": token.args
                    })).unwrap()
                );
            }
            
            let mut table = String::from("<pre>\n");
            table.push_str(&format!("⚙️ Current Settings:\n\n{}\n👤 Minimum Age Required: {}\n💰 Minimum Balance Required: {}", token_info, group.min_approve_age.unwrap_or(0), group.min_approve_balance.unwrap_or(0)));
            table.push_str("</pre>");
            bot.send_message(chat.id, table)
            .parse_mode(ParseMode::Html)
            .await
            .unwrap();
        }
    }
    pub async fn send_list_users_to_admin(&self, bot: Bot, group_id: String, chat: Chat) {
        if let Some(group) = self.tele_dao.get_group(group_id.clone()).await.unwrap() {
            let token_type_hash = group.token_address.clone().unwrap_or("CKB".to_owned());
            let token = self.fetch_token(token_type_hash.clone()).await;
            let members: Vec<TelegramGroupJoined> = self.tele_dao.get_member_by_group(group_id.clone()).await.unwrap_or(vec![]);
            let accepted_count = members.iter().filter(|m| m.status == MEMBER_STATUS_ACCEPTED).count();
            let mut table = String::from(format!("👤 Members: {}/{}\n\n", accepted_count, members.len()));
            if members.is_empty() {
                table.push_str("No one has joined this group.")
            }
            
            for (idx, member) in members.iter().enumerate() {
                if member.status == MEMBER_STATUS_ACCEPTED && token.clone().is_some(){
                    let balances = Value::from_str(&member.balances.clone().unwrap_or("{}".to_owned())).unwrap_or(json!({}));
                    let balance = balances
                        .get(token_type_hash.clone())
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    table.push_str(&format!("{}. {} {} {} (auth: true)", idx+1, member.user_name, token.clone().unwrap().symbol.unwrap_or("Unknown".to_owned()), balance));
                } else {
                    table.push_str(&format!("{}. {} (auth: false)", idx+1, member.user_name));
                }
                
            }
            bot.send_message(chat.id, table)
            .parse_mode(ParseMode::Html)
            .await
            .unwrap();
        }
    }

    pub async fn send_help_to_admin(&self, bot: Bot, chat: Chat) {
        let mut table = String::from("👤 <b>Admin Commands:</b>\n\n");
        table.push_str("1. settoken [type_hash|'ckb'] – set xUDT type\n");
        table.push_str("2. setamount [amount] – set minimum required balance\n");
        table.push_str("3. setage [age] – set minimum required age\n");
        table.push_str("4. groupconfig – view current group settings\n");
        table.push_str("5. listusers – list verified users\n");
        table.push_str("6. sync – sync user lis\n");
        
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
                        .update_member(None, None, member.chat_id, member.user_id, member.expired, MEMBER_STATUS_REJECT, member.balances.unwrap_or("{}".to_owned()))
                        .await;
            }
        }
    }

    pub async fn fetch_token(&self, type_hash: String) -> Option<Token>{
        let type_hash_lowercase = type_hash.to_lowercase();
        if type_hash_lowercase.is_empty() || type_hash_lowercase == "ckb"{
            return Some(Token { 
                type_hash: "".to_owned(),
                name: Some("CKB".to_owned()), 
                symbol: Some("CKB".to_owned()), 
                decimal: Some("6".to_owned()),         
                description: None,
                token_type: 0,
                args: "".to_owned(),
                code_hash: "".to_owned(),
                hash_type: "".to_owned(),
                created_at: Utc::now().naive_utc(), 
                updated_at: Utc::now().naive_utc(),
            });
        }
        
        let token = self.token_dao.get_token(type_hash_lowercase.clone()).await.unwrap();
        if token.is_none() {
            if let Some(token_info) = get_xudt_info(type_hash_lowercase.clone()).await {
                let new_token = Token { 
                    type_hash: type_hash_lowercase,
                    name: token_info.full_name, 
                    symbol: token_info.symbol, 
                    decimal: token_info.decimal, 
                    description: token_info.description, 
                    token_type: TOKEN_TYPE_XUDT, 
                    args: token_info.type_script.clone().unwrap().args, 
                    code_hash: token_info.type_script.clone().unwrap().code_hash, 
                    hash_type: token_info.type_script.clone().unwrap().hash_type, 
                    created_at: Utc::now().naive_utc(), 
                    updated_at: Utc::now().naive_utc()
                };

                let _ = self.token_dao.add_token(new_token.clone()).await;
                return Some(new_token);
            }
        }
        return token
    }
}
