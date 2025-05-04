use std::sync::Arc;

use deadpool_postgres::{Client, Pool, PoolError};
use tokio_pg_mapper::FromTokioPostgresRow;

use crate::models::token::Token;

#[derive(Clone, Debug)]
pub struct TokenDao {
    db: Arc<Pool>,
}

impl TokenDao {
    pub fn new(db: Arc<Pool>) -> Self {
        TokenDao { db: db.clone() }
    }

    pub async fn add_token(&self, token: Token) -> Result<Token, PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt =
            "INSERT INTO tokens (type_hash, name, symbol, decimal, description, token_type, args, code_hash, hash_type) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (type_hash) DO NOTHING";
        let stmt = client.prepare(_stmt).await?;

        client
            .execute(
                &stmt,
                &[
                    &token.type_hash,
                    &token.name,
                    &token.symbol,
                    &token.decimal,
                    &token.description,
                    &token.token_type,
                    &token.args,
                    &token.code_hash,
                    &token.hash_type,
                ],
            )
            .await?;

        Ok(token)
    }

    pub async fn get_token(&self, type_hash: String) -> Result<Option<Token>, PoolError> {
        let client: Client = self.db.get().await?;

        let _stmt = "SELECT * FROM tokens WHERE type_hash=$1;";
        let stmt = client.prepare(_stmt).await?;

        let row = client.query(&stmt, &[&type_hash]).await?.pop();

        Ok(row.map(|row| Token::from_row_ref(&row).unwrap()))
    }
}
