use chrono::NaiveDate;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct VerifyMemberReq {
    pub tgid: i64,
    pub ckb_address: String,
    pub signature: String,
    pub dob: NaiveDate,
    pub sign_type: String,
}
