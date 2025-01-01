use crate::config;
use crate::serialize::error::AppError;
use deadpool_postgres::tokio_postgres::NoTls;
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod};
use once_cell::sync::Lazy;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

pub async fn migrate_db() -> Result<(), AppError> {
    let database_url: String = config::get("database_url");
    let db = PgPoolOptions::new()
        .max_connections(200)
        .connect(&database_url)
        .await
        .map_err(|e| AppError::new(500).message(&e.to_string()))?;

    sqlx::migrate!()
        .run(&db)
        .await
        .map_err(|e| AppError::new(500).message(&e.to_string()))?;

    Ok(())
}

pub static DB_POOL: Lazy<Arc<Pool>> = Lazy::new(|| {
    let mut cfg = Config::new();
    let database_url: String = config::get("database_url");
    cfg.url = Some(database_url);
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let pool = cfg.create_pool(None, NoTls).unwrap();

    Arc::new(pool)
});
