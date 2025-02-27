use std::{sync::Arc, thread, time::Duration};

use utxo_global_tgbot_api::{
    repositories::{db::DB_POOL, member::MemberDao, telegram::TelegramDao},
    services::telegram::TelegramService,
};

async fn run_crons(telegram_svc: Arc<TelegramService>) {
    let time_duration: u64 = 10;
    loop {
        telegram_svc.cron_auto_kick_member().await;
        thread::sleep(Duration::from_secs(time_duration));
    }
}

#[tokio::main]
async fn main() {
    let db = &DB_POOL.clone();
    let member_dao = Arc::new(MemberDao::new(db.clone()));
    let telegram_dao = Arc::new(TelegramDao::new(db.clone()));

    // Initialize the bot
    let telegram_srv = Arc::new(TelegramService::new(
        member_dao.clone(),
        telegram_dao.clone(),
    ));

    println!("Crons is running...");
    run_crons(telegram_srv).await
}
