use std::{str::FromStr, sync::Arc};

use chrono::{NaiveDateTime, Utc};
use teloxide::{
    dispatching::dialogue::GetChatId, payloads::{BanChatMemberSetters, SendMessageSetters}, prelude::*, types::{Chat, ChatKind, ChatMemberStatus, ChatPermissions, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageKind, ParseMode}, utils::command::BotCommands, Bot
};

use crate::{config::{self, MEMBER_BAN_DURATION, MEMBER_KYC_DURATION}, models::{telegram::{TelegramGroup, TelegramGroupAdmin, TelegramGroupJoined, MEMBER_STATUS_ACCEPTED, MEMBER_STATUS_PENDING, MEMBER_STATUS_REJECT}, token::{Token, TOKEN_TYPE_SPORE, TOKEN_TYPE_XUDT}}, repositories::{ckb::{get_collection_info, get_xudt_info}, member::MemberDao, telegram::TelegramDao, token::TokenDao}};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum CommandType {
    SetToken(String),
    SetAmount(i64),
    SetAge(i32),
    GroupConfig,
    ListUsers,
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
        println!("Telegram Bot Running....");
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
            // if telegram user is admin, then add him/her to group_admins
            self.update_group_admin(bot.clone(), chat.clone()).await;

            // create a new group if not exist            
            let group = self.get_group_or_create(chat.clone()).await.unwrap();
            
            for user in msg.clone().new_chat_members {
                if user.is_bot {
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
                
                // send welcome message
                if let Err(err) = bot
                    .send_message(
                        message.chat_id().unwrap(),
                        format!(
                            "Hello @{tgname}, welcome to the group! 👋\nPlease complete your information to get started.\n"
                        ),
                    ).parse_mode(ParseMode::Html)
                    .reply_markup(keyboard.clone())
                    .await
                {
                    log::error!(
                        "Could not message {tgname} (ID: {tgid}). Error: {:?}",
                        err
                    );
                } else {
                    // add new member
                    if let Err(err) = self.member_dao.insert_member(tgid.0 as i64, tgname.clone()).await
                    {
                        log::error!("insert new member failed: {:?}", err);
                    }

                    // add new member to telegram group
                    let member_joined =self.tele_dao.get_member(chat.id.to_string(), tgid.0 as i64).await.unwrap();
                    let expired = Utc::now().naive_utc() + MEMBER_KYC_DURATION;
                    if member_joined.is_none() {
                        let _ = self.tele_dao.add_member(TelegramGroupJoined{
                            chat_id: chat.id.to_string(), 
                            user_id: tgid.0 as i64,
                            user_name: tgname.clone(), 
                            ckb_address: None,
                            dob: None,
                            status: MEMBER_STATUS_PENDING,
                            balances: Some("{}".to_owned()),
                            expired,
                            created_at: Utc::now().naive_utc(), 
                            updated_at: Utc::now().naive_utc() 
                        }).await;
                    } else {
                        let _ = self.tele_dao.update_member(None, None, chat.id.to_string(), tgid.0 as i64, expired, MEMBER_STATUS_PENDING, "{}".to_owned()).await;
                    }
                }

                // send current group settings
                if let Err(err) = bot
                    .send_message(
                        message.chat_id().unwrap(),
                        self.render_group_config(group.clone()).await,
                        )
                    .parse_mode(ParseMode::MarkdownV2)
                    .await
                {
                    log::error!(
                        "Could not message {tgname} (ID: {tgid}). Error: {:?}",
                        err
                    );
                }
                
            }
        } else if text.starts_with("/"){
            if let Ok(command) = CommandType::parse(text, "bot") {
                self.handle_command(bot, message.clone(), command).await;
            }
        }
    }

    async fn render_group_config(&self, group: TelegramGroup) -> String {
        let mut token_info: String = "".to_owned();
        if let Some(type_hash) = group.token_address {
            if let Some(token) = self.fetch_token(type_hash).await {
                token_info = format!(
                    "📦 Token Gating: {}\n🔹 Type Hash: {}\n", 
                    token.name.unwrap(),
                    token.type_hash
                );
            }
        } else {
            token_info = String::from("📦 Token Gating: CKB\n");
        }
        
        let mut table = String::from("").to_owned();
        table.push_str("\n\n⚙️ Current Settings \\(Admin Only\\)\n\n");
        table.push_str(&token_info.to_string());
        table.push_str(&format!("👤 Minimum Age: {}\n💰 Minimum Balance: {}\n", group.min_approve_age.unwrap_or(0), group.min_approve_balance.unwrap_or(0)));
        table
    }

    pub async fn handle_command(&self, bot: &Bot, message: Message, command: CommandType) {
        let is_admin = self.is_admin(message.clone(), bot).await;
        let chat = message.chat.clone();

        // the `/` commands are only for admin
        if !is_admin {
            bot.send_message(
                chat.id,
                "❌ You need “Ban Users” permission to view group settings.",
            )
            .await
            .unwrap();

            let _ = bot.delete_message(chat.id, message.id).await;
            return
        }

        if let Some(mut group) = self.get_group_or_create(chat.clone()).await {
            match command {
                CommandType::SetToken(type_hash) => {
                    group.token_address = Some(type_hash.clone().to_lowercase()); 
                    if let Some(token) = self.fetch_token(type_hash).await {
                        group.token_address = Some(token.type_hash.clone());
                        let is_updated = self.tele_dao.update_group(&group).await.unwrap_or(false);

                        let token_name = token
                                                    .name                       
                                                    .clone()                    
                                                    .unwrap_or_else(|| "Unknown".to_string());
                        let reply = if is_updated {
                            format!("🟢 **Update token: {} successfully!**", token_name)
                        } else {
                            "🔴 **Update token failed!**\nPlease try again later or contact admin for support."
                                .to_string()
                        };

                        bot.send_message(
                            chat.id,
                            reply,
                        )
                        .await
                        .unwrap();
                    } else {
                        bot.send_message(
                            chat.id,
                            "🔴 **Update token failed!**\n Invalid Type Hash",
                        )
                        .await
                        .unwrap();
                    }
                }
                CommandType::SetAmount(amount) => {
                    group.min_approve_balance = Some(amount);
                    match self.tele_dao.update_group(&group).await {
                        Ok(_) => {
                            bot.send_message(chat.id, "✅ Group settings updated successfully\\.")
                                .parse_mode(ParseMode::MarkdownV2)
                                .await
                                .unwrap();
                        },
                        Err(err) => {
                            let err_text = format!("⚠️ Failed to update group settings:\n`{}`", &err.to_string());
                            bot.send_message(chat.id, err_text)
                                .parse_mode(ParseMode::MarkdownV2)
                                .await
                                .unwrap();
                        },
                    }
                }
                CommandType::SetAge(age) => {
                    group.min_approve_age = Some(age);
                    match self.tele_dao.update_group(&group).await {
                        Ok(_) => {
                            bot.send_message(chat.id, "✅ Group settings updated successfully\\.")
                                .parse_mode(ParseMode::MarkdownV2)
                                .await
                                .unwrap();
                        }
                        Err(err) => {
                            let err_text = format!("⚠️ Failed to update group settings:\n`{}`", &err.to_string());
                            bot.send_message(chat.id, err_text)
                                .parse_mode(ParseMode::MarkdownV2)
                                .await
                                .unwrap();
                        }
                    } 
                }
                CommandType::GroupConfig => {
                    self.send_group_config_to_admin(bot.clone(), group.chat_id, chat).await;
                },
                CommandType::ListUsers => {
                    self.send_list_users_to_admin(bot.clone(), group.chat_id, chat).await;
                },
                CommandType::Help => {
                    self.send_help_to_admin(bot.clone(), chat).await;
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
        if let Some(group) = self.tele_dao.get_group(group_id.clone()).await.unwrap() {

            let mut table = self.render_group_config(group.clone()).await;
            let members: Vec<TelegramGroupJoined> = self.tele_dao.get_member_by_group(group_id.clone()).await.unwrap_or(vec![]);
            let accepted_count = members.iter().filter(|m| m.status == MEMBER_STATUS_ACCEPTED).count();
            
            table.push_str(&format!("👥 Verification Status: {}/{} members verified", accepted_count, members.len()));

            bot.send_message(chat.id, table)
            .parse_mode(ParseMode::MarkdownV2)
            .await
            .unwrap();
        }
    }
    
    pub async fn send_list_users_to_admin(&self, bot: Bot, group_id: String, chat: Chat) {
        if self.tele_dao.get_group(group_id.clone()).await.unwrap().is_some(){
            let members: Vec<TelegramGroupJoined> = self.tele_dao.get_member_by_group(group_id.clone()).await.unwrap_or(vec![]);
            let accepted_count = members.iter().filter(|m| m.status == MEMBER_STATUS_ACCEPTED).count();
            
            let mut table = format!("👥 Verification Status: {}/{} members verified\n\n", accepted_count, members.len());
            if members.is_empty() {
                table.push_str("No one has joined this group.")
            }
            
            for (idx, member) in members.iter().enumerate() {
                if member.status == MEMBER_STATUS_ACCEPTED {
                    table.push_str(&format!("{}. @{} (auth: true)\n", idx+1, member.user_name));
                } else {
                    table.push_str(&format!("{}. @{} (❌ Not verified yet)\n", idx+1, member.user_name));
                }
                
            }
            bot.send_message(chat.id, table)
            .parse_mode(ParseMode::Html)
            .await
            .unwrap();
        }
    }

    pub async fn send_help_to_admin(&self, bot: Bot, chat: Chat) {
        let mut table = String::from("*👤 Admin Commands:*\n\n");
        table.push_str("1\\. `/settoken (type_script_hash|ckb)`: Set the gated token\n");
        table.push_str("2\\. `/setamount (amount)`: Set minimum required balance\n");
        table.push_str("3\\. `/setage (age)`: Set minimum required age \\(years\\)\n");
        table.push_str("4\\. `/groupconfig`: View current group settings\n");
        table.push_str("5\\. `/listusers`: List currently verified users\n");
        table.push_str("6\\. `/mygroups`: Bot status: list groups the bot manages\n");
        
        bot.send_message(chat.id, table)
        .parse_mode(ParseMode::MarkdownV2)
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
                                "🔴 **{}** failed verification and was removed.\n\
                                _Reason:_ didn’t complete verification within 5 minutes.\n\
                                They can rejoin and try again after the 15‑minute cooldown.",
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
        let type_hash = type_hash.to_lowercase();
        let now = Utc::now().naive_utc();
        if type_hash.is_empty() || type_hash == "ckb" {
            return Some(Token::ckb(now));
        }

        if let Ok(Some(token)) = self.token_dao.get_token(type_hash.clone()).await {
            return Some(token);
        }

        self.fetch_and_store(type_hash, now).await
    }

    async fn fetch_and_store(&self, type_hash: String, now: NaiveDateTime) -> Option<Token> {
        // Try XUDT
        if let Some(info) = get_xudt_info(type_hash.clone()).await {
            if let Some(ts) = info.type_script {
                let tok = Token {
                    type_hash:   type_hash.clone(),
                    name:        info.full_name,
                    symbol:      info.symbol,
                    decimal:     info.decimal,
                    description: info.description,
                    token_type:  TOKEN_TYPE_XUDT,
                    args:        ts.args.unwrap_or("".to_owned()),
                    code_hash:   ts.code_hash.unwrap_or("".to_owned()),
                    hash_type:   ts.hash_type.unwrap_or("".to_owned()),
                    created_at:  now,
                    updated_at:  now,
                };
                let _ = self.token_dao.add_token(tok.clone()).await;
                return Some(tok);
            }
        }

        // Fallback to collection 
        if let Some(info) = get_collection_info(type_hash.clone()).await {
            let ts = info.type_script;
            let tok = Token {
                type_hash:   type_hash.clone(),
                name:        Some(info.name),
                symbol:      Some(String::new()),
                decimal:     Some(String::new()),
                description: Some(info.standard),
                token_type:  TOKEN_TYPE_SPORE,
                args:        ts.args,
                code_hash:   ts.code_hash,
                hash_type:   ts.hash_type,
                created_at:  now,
                updated_at:  now,
            };
            let _ = self.token_dao.add_token(tok.clone()).await;
            return Some(tok);
        }

        // Nothing found
        None
    }
}
