// Signer Type: BtcEcdsa

use super::types;
use bitcoin::{
    sign_message::{signed_msg_hash, MessageSignature},
    PublicKey,
};
use secp256k1::Secp256k1;

pub fn verify_message(challenge: &str, data: types::SignData) -> bool {
    let public_key_bytes =
        hex::decode(data.identity.as_str().replace("0x", "")).expect("Failed to decode pubkey");
    let public_key = PublicKey::from_slice(&public_key_bytes).expect("Invalid public key");
    let secp = Secp256k1::new();
    let msg_hash = signed_msg_hash(challenge);
    let msg_sig =
        MessageSignature::from_base64(data.signature.as_str()).expect("Invalid signature");
    let recover_public_key = msg_sig
        .recover_pubkey(&secp, msg_hash)
        .expect("recovery pubkey failed");

    recover_public_key.inner.to_string() == public_key.inner.to_string()
}
