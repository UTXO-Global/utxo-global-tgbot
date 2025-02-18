use std::str::FromStr;

use chrono::{Datelike, NaiveDate, Utc};
use ckb_hash::{Blake2bBuilder, CKB_HASH_PERSONALIZATION};
use ckb_sdk::{Address, AddressPayload};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1,
};
use serde_json::Value;
use teloxide::{
    prelude::Requester,
    types::{ChatPermissions, UserId},
    Bot,
};

use crate::{
    config,
    models::telegram::{MEMBER_STATUS_ACCEPTED, MEMBER_STATUS_PENDING, MEMBER_STATUS_REJECT},
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
        match self
            .tele_dao
            .get_group_by_user_id(req.tgid, Some(MEMBER_STATUS_PENDING))
            .await
        {
            Ok(groups) => {
                let balances = get_balances(req.ckb_address).await;
                let age = self.calc_age(req.dob.clone());
                let bot_token: String = config::get("bot_token");
                let bot = Bot::new(bot_token);
                for group in groups {
                    if group.status == MEMBER_STATUS_ACCEPTED {
                        continue;
                    }

                    let balance = balances.get("CKB").and_then(Value::as_u64).unwrap_or(0);
                    if balance > 150 && age > 17 {
                        // from 18 age above
                        let _ = bot
                            .restrict_chat_member(
                                group.clone().chat_id.to_string(),
                                UserId(group.clone().user_id as u64),
                                ChatPermissions::all(),
                            )
                            .await;
                        bot.send_message(
                            group.clone().chat_id.to_string(),
                            format!("✅ User {} approved!", group.clone().user_name),
                        )
                        .await
                        .unwrap();
                        let _ = self
                            .tele_dao
                            .update_mmember(group.chat_id, group.user_id, MEMBER_STATUS_ACCEPTED)
                            .await;
                    } else {
                        let _ = bot
                            .ban_chat_member(
                                group.clone().chat_id.to_string(),
                                UserId(group.clone().user_id as u64),
                            )
                            .await;

                        let reason = if age < 18 {
                            "Under 18 years odl"
                        } else {
                            "Insufficient balance(Min: 150 KCB)"
                        };

                        bot.send_message(
                            group.clone().chat_id.to_string(),
                            format!(
                                "⚠️ User {} banned! \nReason: {}",
                                group.clone().user_name,
                                reason
                            ),
                        )
                        .await
                        .unwrap();

                        let _ = self
                            .tele_dao
                            .update_mmember(
                                group.chat_id.clone(),
                                group.user_id.clone(),
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

        return age;
    }
}
