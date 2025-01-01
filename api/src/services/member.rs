use std::str::FromStr;

use chrono::NaiveDate;
use ckb_hash::{Blake2bBuilder, CKB_HASH_PERSONALIZATION};
use ckb_sdk::{Address, AddressPayload};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1,
};

use crate::{
    repositories::{ckb::get_ckb_network, member::MemberDao},
    serialize::{error::AppError, member::VerifyMemberReq},
};

#[derive(Clone, Debug)]
pub struct MemberSrv {
    member_dao: MemberDao,
}

impl MemberSrv {
    pub fn new(member_dao: MemberDao) -> Self {
        MemberSrv {
            member_dao: member_dao.clone(),
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
        let signature = req.signature;
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
            Ok(())
        } else {
            Err(AppError::new(500).message("Signature not matched"))
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
}
