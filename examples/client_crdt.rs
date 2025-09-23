use anyhow::Result;
use log::{error, info, LevelFilter};

use sink::SyncClient;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.as_str()),
    )
    .init();

    let mut client = SyncClient::new(String::from("test 1"));
    info!("Connecting to server...");

    match client.connect("ws://0.0.0.0:5000").await {
        Ok(_) => info!("✓ Connected successfully!"),
        Err(e) => {
            error!("✗ Failed to connect: {}", e);
            error!("Make sure the server is running with: cargo run server");
            return Ok(());
        }
    }

    client.start().await.unwrap();

    Ok(())
}
