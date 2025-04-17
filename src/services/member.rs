use crate::{
    config::{self, MEMBER_BAN_DURATION},
    libs::signer::{types, verify},
    models::telegram::{
        TelegramGroup, MEMBER_STATUS_ACCEPTED, MEMBER_STATUS_PENDING, MEMBER_STATUS_REJECT,
    },
    repositories::{ckb::get_balances, member::MemberDao, telegram::TelegramDao},
    serialize::{error::AppError, member::VerifyMemberReq},
};

use chrono::{Datelike, NaiveDate, Utc};
use serde_json::Value;
use std::collections::HashMap;
use teloxide::{
    payloads::BanChatMemberSetters,
    prelude::Requester,
    types::{ChatPermissions, UserId},
    Bot,
};

#[derive(Clone, Debug)]
pub struct MemberSrv {
    member_dao: MemberDao,
    tele_dao: TelegramDao,
}

impl MemberSrv {
    pub fn new(member_dao: MemberDao, tele_dao: TelegramDao) -> Self {
        MemberSrv {
            member_dao: member_dao.clone(),
            tele_dao: tele_dao.clone(),
        }
    }

    pub async fn verify_signature(&self, req: VerifyMemberReq) -> Result<(), AppError> {
        let challenge = format!("My tgid: {} - My DoB: {}", req.tgid, req.dob);
        let mut sign_data = types::SignData::from(&req.signature);
        sign_data.ckb_address = Some(req.ckb_address.clone());

        if verify::verify_message(&challenge, sign_data) {
            let _ = self.verify_info(req).await;
            Ok(())
        } else {
            Err(AppError::new(500).message("Signature not matched"))
        }
    }

    pub async fn verify_info(&self, req: VerifyMemberReq) {
        let mut groups: HashMap<String, TelegramGroup> = HashMap::new();
        match self
            .tele_dao
            .get_group_by_user_id(req.tgid, Some(MEMBER_STATUS_PENDING))
            .await
        {
            Ok(joined_groups) => {
                let balances = get_balances(req.ckb_address.clone()).await;
                let age = self.calc_age(req.dob);
                let bot_token: String = config::get("bot_token");
                let bot = Bot::new(bot_token);
                for member in joined_groups {
                    if member.status == MEMBER_STATUS_ACCEPTED {
                        continue;
                    }

                    let group: Option<TelegramGroup> = if let Some(existing_group) =
                        groups.get(&member.chat_id)
                    {
                        Some(existing_group.clone())
                    } else {
                        match self.tele_dao.get_group(member.chat_id.clone()).await {
                            Ok(fetched_group) => {
                                groups
                                    .insert(member.chat_id.clone(), fetched_group.clone().unwrap());
                                fetched_group
                            }
                            Err(_) => None,
                        }
                    };

                    if group.is_none() {
                        println!("Group {} not existed", member.chat_id);
                        continue;
                    }

                    let group_unwrap = group.clone().unwrap();
                    let min_age_approved = group_unwrap.clone().min_approve_age.unwrap_or(0);
                    let min_balance_approved =
                        group_unwrap.clone().min_approve_balance.unwrap_or(0) as f64;
                    let token_address = group_unwrap
                        .clone()
                        .token_address
                        .unwrap_or("CKB".to_owned());

                    let balance = balances
                        .get(token_address)
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);

                    if balance >= min_balance_approved && age >= min_age_approved {
                        let _ = bot
                            .restrict_chat_member(
                                member.clone().chat_id.to_string(),
                                UserId(member.clone().user_id as u64),
                                ChatPermissions::all(),
                            )
                            .await;
                        bot.send_message(
                            member.clone().chat_id.to_string(),
                            format!(
                                "🟢 **Verification successful!**\n\
                                Welcome, **{}** — you now have full access. Enjoy the chat! 🎉",
                                member.clone().user_name
                            ),
                        )
                        .await
                        .unwrap();

                        let _ = self
                            .tele_dao
                            .update_member(
                                Some(req.ckb_address.clone()),
                                Some(req.dob),
                                member.chat_id,
                                member.user_id,
                                member.expired,
                                MEMBER_STATUS_ACCEPTED,
                                balances.to_string(),
                            )
                            .await;
                    } else {
                        let until_date = Utc::now() + MEMBER_BAN_DURATION;
                        let _ = bot
                            .ban_chat_member(
                                member.clone().chat_id.to_string(),
                                UserId(member.clone().user_id as u64),
                            )
                            .until_date(until_date)
                            .await;

                        let reason = if age < min_age_approved {
                            format!("Under {} years odl", min_age_approved)
                        } else {
                            format!(
                                "Insufficient balance(Min: {} {})",
                                min_balance_approved.clone(),
                                group_unwrap
                                    .clone()
                                    .token_address
                                    .unwrap_or("CKB".to_string())
                            )
                        };

                        let _ = bot
                            .send_message(
                                member.clone().chat_id.to_string(),
                                format!(
                                    "🔴 **{}** failed verification and was removed.\n\
                                    _Reason:_ {}.\n\
                                    They can rejoin and try again after the 15‑minute cooldown.",
                                    member.clone().user_name,
                                    reason
                                ),
                            )
                            .await;

                        let _ = self
                            .tele_dao
                            .update_member(
                                Some(req.ckb_address.clone()),
                                Some(req.dob),
                                member.chat_id,
                                member.user_id,
                                member.expired,
                                MEMBER_STATUS_REJECT,
                                balances.to_string(),
                            )
                            .await;
                    }
                }
            }
            Err(err) => {
                println!("{:?}", err)
            }
        }
    }

    pub async fn update_member(
        &self,
        tgid: i64,
        user_address: String,
        balance: pg_bigdecimal::PgNumeric,
        dob: NaiveDate,
        status: i16,
    ) -> Result<(), AppError> {
        self.member_dao
            .update_member(tgid, user_address, balance, dob, status)
            .await
            .map_err(|e| AppError::new(500).cause(e).message("update member failed"))
    }

    fn calc_age(&self, dob: NaiveDate) -> i32 {
        let today = Utc::now().date_naive();
        let mut age: i32 = today.year() - dob.year();
        if today.month0() < dob.month0() && today.day0() < dob.day0() {
            age -= 1i32;
        }

        age
    }
}
