use chrono::NaiveDateTime;
use serde_derive::{Deserialize, Serialize};
use tokio_pg_mapper_derive::PostgresMapper;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TokenType {
    Xudt,
    Spore,
}

pub const TOKEN_TYPE_XUDT: i16 = TokenType::Xudt as i16;
pub const TOKEN_TYPE_SPORE: i16 = TokenType::Spore as i16;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PostgresMapper)]
#[pg_mapper(table = "tokens")]
pub struct Token {
    pub type_hash: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimal: Option<String>,
    pub description: Option<String>,
    pub token_type: i16,
    pub args: String,
    pub code_hash: String,
    pub hash_type: String,

    #[serde(skip_serializing)]
    pub created_at: NaiveDateTime,

    #[serde(skip_serializing)]
    pub updated_at: NaiveDateTime,
}

impl Token {
    /// Construct the built-in CKB token
    pub fn ckb(now: NaiveDateTime) -> Self {
        Token {
            type_hash: String::new(),
            name: Some("CKB".into()),
            symbol: Some("CKB".into()),
            decimal: Some("6".into()),
            description: None,
            token_type: 0,
            args: String::new(),
            code_hash: String::new(),
            hash_type: String::new(),
            created_at: now,
            updated_at: now,
        }
    }
}
