use std::io;
use utxo_global_tgbot_api::app;

#[tokio::main]
async fn main() -> io::Result<()> {
    app::create_app().await
}
