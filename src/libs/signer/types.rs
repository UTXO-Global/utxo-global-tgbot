use serde::Deserialize;

pub const BTC_ECDSA: &str = "btcecdsa";
pub const EVM_PERSONAL: &str = "evmpersonal";
pub const JOY_ID: &str = "joyid";
pub const CKB_SECP256K1: &str = "ckbsecp256k1";
pub const DOGE_ECDSA: &str = "dogeecdsa";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignData {
    pub signature: String,
    pub identity: String,
    pub sign_type: String,
    pub ckb_address: Option<String>,
}

impl SignData {
    pub fn from(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}
