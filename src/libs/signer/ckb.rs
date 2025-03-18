// Signer Type: CkbSecp256k1

use ckb_hash::{Blake2bBuilder, CKB_HASH_PERSONALIZATION};
use ckb_sdk::{Address, AddressPayload};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1,
};

use std::str::FromStr;

use crate::repositories::ckb::get_ckb_network;

use super::types::SignData;

fn hash_ckb(message: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2bBuilder::new(32)
        .personal(CKB_HASH_PERSONALIZATION)
        .build();
    hasher.update(message);
    let mut result = [0; 32];
    hasher.finalize(&mut result);
    result
}

pub fn verify_signature(challenge: &str, data: SignData) -> bool {
    let signature = data.signature.clone().replace("0x", "");
    let message = format!("Nervos Message:{}", challenge);
    let message_hash: [u8; 32] = hash_ckb(message.as_bytes());
    let secp_message = Message::from_digest_slice(&message_hash).expect("Invalid message hash");

    let sig_bytes: Vec<u8> = hex::decode(signature).expect("Invalid signature hex");
    let r = &sig_bytes[0..32];
    let s = &sig_bytes[32..64];
    let rec_id = sig_bytes[64]; // Recovery ID as byte
    let rec_id = RecoveryId::try_from(rec_id as i32).expect("Invalid recovery ID");
    let mut ret: [u8; 64] = [0; 64];
    ret[..32].copy_from_slice(r);
    ret[32..].copy_from_slice(s);

    let rec_sig =
        RecoverableSignature::from_compact(&ret, rec_id).expect("Invalid recoverable signature");

    let secp = Secp256k1::new();
    let pub_key = secp
        .recover_ecdsa(&secp_message, &rec_sig)
        .expect("Failed to recover public key");

    let pub_key_bytes = pub_key.serialize();
    let expected_pubkey = PublicKey::from_slice(&pub_key_bytes).expect("Invalid public key");
    let address = Address::from_str(&data.ckb_address.unwrap()).unwrap();
    let recovered_address = Address::new(
        get_ckb_network(),
        AddressPayload::from_pubkey(&expected_pubkey),
        true,
    );

    recovered_address.to_string() == address.to_string()
}
