use std::sync::Arc;

use deadpool_postgres::{Client, Pool, PoolError};
use tokio_pg_mapper::FromTokioPostgresRow;

use crate::models::telegram::{TelegramGroup, TelegramGroupJoined};

#[derive(Clone, Debug)]
pub struct TelegramDao {
    db: Arc<Pool>,
}

impl TelegramDao {
    pub fn new(db: Arc<Pool>) -> Self {
        TelegramDao { db: db.clone() }
    }

    pub async fn add_group(&self, group: TelegramGroup) -> Result<TelegramGroup, PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt =
            "INSERT INTO tg_groups (chat_id, name, token_address, min_approve_balance, min_approve_age) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (chat_id) DO NOTHING ;";
        let stmt = client.prepare(_stmt).await?;

        client
            .execute(
                &stmt,
                &[
                    &group.chat_id,
                    &group.name,
                    &group.token_address,
                    &group.min_approve_balance,
                    &group.min_approve_age,
                ],
            )
            .await?;

        Ok(group)
    }

    pub async fn add_member(
        &self,
        member: TelegramGroupJoined,
    ) -> Result<TelegramGroupJoined, PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt =
            "INSERT INTO tg_group_joined (chat_id, user_id, user_name ) VALUES ($1, $2, $3) ON CONFLICT (chat_id, user_id) DO NOTHING ;";
        let stmt = client.prepare(_stmt).await?;

        client
            .execute(
                &stmt,
                &[&member.chat_id, &member.user_id, &member.user_name],
            )
            .await?;

        Ok(member)
    }

    pub async fn update_group(&self, group: &TelegramGroup) -> Result<bool, PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt =
            "UPDATE tg_groups SET token_address=$1, min_approve_balance=$2, min_approve_age=$3 WHERE chat_id=$4;";
        let stmt = client.prepare(_stmt).await?;

        let affected_rows = client
            .execute(
                &stmt,
                &[
                    &group.token_address,
                    &group.min_approve_balance,
                    &group.min_approve_age,
                    &group.chat_id,
                ],
            )
            .await?;

        Ok(affected_rows > 0)
    }

    pub async fn update_mmember(
        &self,
        chat_id: String,
        user_id: i64,
        status: i16,
    ) -> Result<bool, PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt = "UPDATE tg_group_joined SET status=$1 WHERE chat_id=$2 AND user_id=$3";
        let stmt = client.prepare(_stmt).await?;

        let affected_rows = client
            .execute(&stmt, &[&status, &chat_id, &user_id])
            .await?;

        Ok(affected_rows > 0)
    }

    pub async fn get_group(&self, chat_id: String) -> Result<Option<TelegramGroup>, PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt = "SELECT * FROM tg_groups WHERE chat_id=$1;";
        let stmt = client.prepare(_stmt).await?;

        let row = client.query(&stmt, &[&chat_id]).await?.pop();

        Ok(row.map(|row| TelegramGroup::from_row_ref(&row).unwrap()))
    }

    pub async fn get_group_by_user_id(
        &self,
        user_id: i64,
        status: Option<i16>,
    ) -> Result<Vec<TelegramGroupJoined>, PoolError> {
        let client: Client = self.db.get().await?;

        let mut _stmt = String::from("SELECT * FROM tg_group_joined WHERE user_id=$1");
        if let Some(st) = status {
            _stmt += &format!(" AND status={}", st);
        }

        let stmt = client.prepare(&_stmt).await?;

        return Ok(client
            .query(&stmt, &[&user_id])
            .await?
            .iter()
            .map(|row| TelegramGroupJoined::from_row_ref(row).unwrap())
            .collect::<Vec<TelegramGroupJoined>>());
    }

    pub async fn get_member(
        &self,
        chat_id: String,
        user_id: i64,
    ) -> Result<Option<TelegramGroupJoined>, PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt = "SELECT * FROM tg_group_joined WHERE chat_id=$1 AND user_id=$2;";
        let stmt = client.prepare(_stmt).await?;

        let row = client.query(&stmt, &[&chat_id, &user_id]).await?.pop();
        Ok(row.map(|row| TelegramGroupJoined::from_row_ref(&row).unwrap()))
    }
}
