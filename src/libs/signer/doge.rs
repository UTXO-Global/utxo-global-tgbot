// Signer Type: DogeEcdsa

use super::types;
use base64::{engine::general_purpose::STANDARD, Engine};
use bitcoin::{
    consensus::{encode as ConsensusEncode, Encodable as _},
    hashes::{
        ripemd160::Hash as Ripemd160Hash, sha256::Hash as Sha256Hash, sha256d, Hash,
        HashEngine as _,
    },
};
use secp256k1::{ecdsa::RecoverableSignature, Message, Secp256k1};

pub fn signed_msg_hash(msg: &str) -> sha256d::Hash {
    let mut engine = sha256d::Hash::engine();
    engine.input(b"\x19Dogecoin Signed Message:\n");
    let msg_len = ConsensusEncode::VarInt::from(msg.len());
    msg_len
        .consensus_encode(&mut engine)
        .expect("engines don't error");
    engine.input(msg.as_bytes());
    sha256d::Hash::from_engine(engine)
}

fn verify_message_doge_ecdsa(message: &str, signature: &str, address: &str) -> bool {
    let secp = Secp256k1::new();
    let signature_bytes = STANDARD.decode(signature).expect("Decode signature failed");

    if signature_bytes.is_empty() {
        return false;
    }

    let recovery_bit = signature_bytes[0];
    let raw_sign = &signature_bytes[1..];

    let rec_id = match secp256k1::ecdsa::RecoveryId::try_from((recovery_bit - 31) as i32) {
        Ok(id) => id,
        Err(_) => return false,
    };

    let recoverable_sig = match RecoverableSignature::from_compact(raw_sign, rec_id) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let challenge = signed_msg_hash(message);

    let msg: Message = match Message::from_digest_slice(&challenge.to_byte_array()) {
        Ok(m) => m,
        Err(_) => return false,
    };

    let recovered_pubkey = match secp.recover_ecdsa(&msg, &recoverable_sig) {
        Ok(pk) => pk,
        Err(_) => return false,
    };

    let pubkey_bytes = recovered_pubkey.serialize();
    let pubkey_hash = Ripemd160Hash::hash(&Sha256Hash::hash(&pubkey_bytes).to_byte_array());

    let expected_hash = match btc_public_key_from_p2pkh_address(address) {
        Some(hash) => hash,
        None => {
            return false;
        }
    };

    expected_hash == hex::encode(pubkey_hash)
}

fn btc_public_key_from_p2pkh_address(address: &str) -> Option<String> {
    let decoded = match bs58::decode(address).with_check(None).into_vec() {
        Ok(bytes) => bytes,
        Err(_) => return None,
    };

    if decoded.len() < 2 {
        return None;
    }
    Some(hex::encode(&decoded[1..]))
}

pub fn verify_message(challenge: &str, data: types::SignData) -> bool {
    verify_message_doge_ecdsa(challenge, &data.signature, &data.identity)
}
