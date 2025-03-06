use super::{btc, ckb, doge, evm, joyid, types};

pub fn verify_message(challenge: &str, data: types::SignData) -> bool {
    let sign_type = data.sign_type.to_lowercase();
    match sign_type.as_str() {
        types::BTC_ECDSA => return btc::verify_message(challenge, data),
        types::EVM_PERSONAL => return evm::verify_message(challenge, data),
        types::JOY_ID => return joyid::verify_signature(challenge, data),
        types::CKB_SECP256K1 => return ckb::verify_signature(challenge, data),
        types::DOGE_ECDSA => return doge::verify_message(challenge, data),
        _ => return false,
    }
}
