use chrono::NaiveDateTime;
use serde_derive::{Deserialize, Serialize};
use tokio_pg_mapper_derive::PostgresMapper;

pub enum GroupMemberStatus {
    Pending,
    Accepted,
    Rejected,
}

pub const MEMBER_STATUS_PENDING: i16 = GroupMemberStatus::Pending as i16;
pub const MEMBER_STATUS_ACCEPTED: i16 = GroupMemberStatus::Accepted as i16;
pub const MEMBER_STATUS_REJECT: i16 = GroupMemberStatus::Rejected as i16;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PostgresMapper)]
#[pg_mapper(table = "tg_groups")]
pub struct TelegramGroup {
    pub chat_id: String,
    pub name: String,
    pub status: i16,
    pub token_address: Option<String>,
    pub min_approve_balance: Option<i64>,
    pub min_approve_age: Option<i32>,

    #[serde(skip_serializing)]
    pub created_at: NaiveDateTime,

    #[serde(skip_serializing)]
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PostgresMapper)]
#[pg_mapper(table = "tg_group_joined")]
pub struct TelegramGroupJoined {
    pub chat_id: String,
    pub user_id: i64,
    pub user_name: String,
    pub status: i16,
    pub expired: NaiveDateTime,

    #[serde(skip_serializing)]
    pub created_at: NaiveDateTime,

    #[serde(skip_serializing)]
    pub updated_at: NaiveDateTime,
}
