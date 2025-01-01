use chrono::{NaiveDate, NaiveDateTime};
use pg_bigdecimal::PgNumeric;
use serde_derive::{Deserialize, Serialize};
use tokio_pg_mapper_derive::PostgresMapper;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PostgresMapper)]
#[pg_mapper(table = "users")]
pub struct Member {
    pub tgid: i64,
    pub tgname: Option<String>,
    pub status: i16,
    pub ckb_address: Option<String>,
    pub balance: Option<PgNumeric>,
    pub dob: Option<NaiveDate>,

    #[serde(skip_serializing)]
    pub created_at: NaiveDateTime,
}
