use std::sync::Arc;

use chrono::NaiveDate;
use deadpool_postgres::{Client, Pool, PoolError};

#[derive(Clone, Debug)]
pub struct MemberDao {
    db: Arc<Pool>,
}

impl MemberDao {
    pub fn new(db: Arc<Pool>) -> Self {
        MemberDao { db: db.clone() }
    }

    pub async fn update_member(
        &self,
        tgid: i64,
        ckb_address: String,
        balance: pg_bigdecimal::PgNumeric,
        dob: NaiveDate,
        status: i16,
    ) -> Result<(), PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt = "UPDATE members SET ckb_address = $2, balance = $3, dob = $4, status = $5 WHERE tgid = $1;";
        let stmt = client.prepare(_stmt).await?;

        client
            .execute(&stmt, &[&tgid, &ckb_address, &balance, &dob, &status])
            .await?;
        Ok(())
    }

    pub async fn insert_member(&self, tgid: i64, tgname: String) -> Result<(), PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt =
            "INSERT INTO members (tgid, tgname) VALUES ($1, $2) ON CONFLICT (tgid) DO NOTHING ;";
        let stmt = client.prepare(_stmt).await?;

        client.execute(&stmt, &[&tgid, &tgname]).await?;
        Ok(())
    }
}
