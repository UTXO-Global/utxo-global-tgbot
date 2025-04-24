use std::sync::Arc;

use utxo_global_tgbot_api::{
    repositories::{db::DB_POOL, member::MemberDao, telegram::TelegramDao, token::TokenDao},
    services::telegram::TelegramService,
};

async fn run_crons(telegram_svc: Arc<TelegramService>) {
    /*
    let time_duration: u64 = 10;
    loop {
        telegram_svc.cron_auto_kick_member().await;
        thread::sleep(Duration::from_secs(time_duration));
    }
    */

    telegram_svc.cron_auto_kick_member().await;
}

#[tokio::main]
async fn main() {
    let db = &DB_POOL.clone();
    let member_dao = Arc::new(MemberDao::new(db.clone()));
    let tele_dao = Arc::new(TelegramDao::new(db.clone()));
    let token_dao = Arc::new(TokenDao::new(db.clone()));

    // Initialize the bot
    let telegram_srv = Arc::new(TelegramService::new(
        member_dao.clone(),
        tele_dao.clone(),
        token_dao.clone(),
    ));

    println!("Crons is running...");
    run_crons(telegram_srv).await
}
