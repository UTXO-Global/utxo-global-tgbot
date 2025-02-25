use std::sync::Arc;
use utxo_global_tgbot_api::repositories;
use utxo_global_tgbot_api::repositories::db::{migrate_db, DB_POOL};
use utxo_global_tgbot_api::services::telegram::TelegramService;

#[tokio::main]
async fn main() {
    env_logger::init();
    let db = &DB_POOL.clone();
    let member_dao = Arc::new(repositories::member::MemberDao::new(db.clone()));
    let telegram_dao: Arc<repositories::telegram::TelegramDao> =
        Arc::new(repositories::telegram::TelegramDao::new(db.clone()));

    // migrate db
    if let Err(e) = migrate_db().await {
        println!("\nMigrate db failed: {}", e);
    }

    // Initialize the bot
    let telegram_srv = Arc::new(TelegramService::new(
        member_dao.clone(),
        telegram_dao.clone(),
    ));
    telegram_srv.start().await;
}
