// Signer Type: EvmPersonal

use super::types;
use ethers::{prelude::*, utils::hex};
pub fn verify_message(challenge: &str, data: types::SignData) -> bool {
    let message = format!(
        "\x19Ethereum Signed Message:\n{}{}",
        challenge.len(),
        challenge
    );
    let message_hash = ethers::utils::keccak256(message.clone().as_bytes());

    let sig_bytes = hex::decode(&data.signature[2..]).expect("Failed to decode signature");
    let sig = Signature::try_from(sig_bytes.as_slice()).expect("Failed to parse signature");

    match sig.recover(message_hash) {
        Ok(recovered) => recovered == data.identity.parse::<Address>().unwrap(),
        Err(_) => false,
    }
}
