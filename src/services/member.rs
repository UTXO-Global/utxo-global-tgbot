use std::{collections::HashMap, str::FromStr};

use chrono::{Datelike, NaiveDate, Utc};
use ckb_hash::{Blake2bBuilder, CKB_HASH_PERSONALIZATION};
use ckb_sdk::{Address, AddressPayload};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1,
};
use serde_json::Value;
use teloxide::{
    payloads::BanChatMemberSetters,
    prelude::Requester,
    types::{ChatPermissions, UserId},
    Bot,
};

use crate::{
    config::{self, MEMBER_BAN_DURATION},
    models::telegram::{
        TelegramGroup, MEMBER_STATUS_ACCEPTED, MEMBER_STATUS_PENDING, MEMBER_STATUS_REJECT,
    },
    repositories::{
        ckb::{get_balances, get_ckb_network},
        member::MemberDao,
        telegram::TelegramDao,
    },
    serialize::{error::AppError, member::VerifyMemberReq},
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

    fn hash_ckb(&self, message: &[u8]) -> [u8; 32] {
        let mut hasher = Blake2bBuilder::new(32)
            .personal(CKB_HASH_PERSONALIZATION)
            .build();
        hasher.update(message);
        let mut result = [0; 32];
        hasher.finalize(&mut result);
        result
    }

    pub async fn verify_signature(&self, req: VerifyMemberReq) -> Result<(), AppError> {
        let signature = req.signature.clone();
        let message = format!("Nervos Message:My tgid: {} - My DoB: {}", req.tgid, req.dob);
        let message_hash = self.hash_ckb(message.as_bytes());
        let secp_message = Message::from_digest_slice(&message_hash).expect("Invalid message hash");

        let sig_bytes = hex::decode(signature).expect("Invalid signature hex");
        let r = &sig_bytes[0..32];
        let s = &sig_bytes[32..64];
        let rec_id = sig_bytes[64]; // Recovery ID as byte
        let rec_id = RecoveryId::from_i32(rec_id as i32).expect("Invalid recovery ID");
        let mut ret: [u8; 64] = [0; 64];
        ret[..32].copy_from_slice(r);
        ret[32..].copy_from_slice(s);

        let rec_sig = RecoverableSignature::from_compact(&ret, rec_id)
            .expect("Invalid recoverable signature");

        let secp = Secp256k1::new();
        let pub_key = secp
            .recover_ecdsa(&secp_message, &rec_sig)
            .expect("Failed to recover public key");

        let pub_key_bytes = pub_key.serialize();
        let expected_pubkey = PublicKey::from_slice(&pub_key_bytes).expect("Invalid public key");
        let address = Address::from_str(&req.ckb_address).unwrap();
        let recovered_address = Address::new(
            get_ckb_network(),
            AddressPayload::from_pubkey(&expected_pubkey),
            true,
        );

        if recovered_address.to_string() == address.to_string() {
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
                let balances = get_balances(req.ckb_address).await;
                let age = self.calc_age(req.dob);
                let bot_token: String = config::get("bot_token");
                let bot = Bot::new(bot_token);
                for member in joined_groups {
                    println!("member {:?}", member);
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
                            format!("✅ User {} approved!", member.clone().user_name),
                        )
                        .await
                        .unwrap();
                        let _ = self
                            .tele_dao
                            .update_mmember(
                                member.chat_id,
                                member.user_id,
                                member.expired,
                                MEMBER_STATUS_ACCEPTED,
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
                                    "⚠️ User {} banned! \nReason: {}",
                                    member.clone().user_name,
                                    reason
                                ),
                            )
                            .await;

                        let _ = self
                            .tele_dao
                            .update_mmember(
                                member.chat_id,
                                member.user_id,
                                member.expired,
                                MEMBER_STATUS_REJECT,
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
