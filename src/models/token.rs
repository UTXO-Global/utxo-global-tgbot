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
