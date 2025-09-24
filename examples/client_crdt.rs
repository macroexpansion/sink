use anyhow::Result;
use log::{error, info, LevelFilter};
use tokio::time::{sleep, Duration};

use sink::{ClientRequest, SyncClient};

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
    let request_tx = client.request_tx.clone();

    let start_task = client.start();

    let send_data_task = tokio::spawn(async move {
        loop {
            let duration_a = Duration::from_secs(rand::random_range(2..5));
            let duration_b = Duration::from_secs(rand::random_range(2..5));

            tokio::select! {
                _ = sleep(duration_a) => {
                    request_tx.send(ClientRequest::Sync).unwrap();
                }
                _ = sleep(duration_b) => {
                    request_tx.send(ClientRequest::Update {
                        content: "Hello, world!".to_string(),
                    }).unwrap();
                }
            }
        }
    });

    _ = tokio::join!(send_data_task, start_task);

    Ok(())
}
