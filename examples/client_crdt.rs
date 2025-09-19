use anyhow::Result;

use sink::SyncClient;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = SyncClient::new(String::from("test 1"));
    println!("Connecting to server...");

    match client.connect("ws://0.0.0.0:5000").await {
        Ok(_) => println!("✓ Connected successfully!"),
        Err(e) => {
            println!("✗ Failed to connect: {}", e);
            println!("Make sure the server is running with: cargo run server");
            return Ok(());
        }
    }

    client.start().await.unwrap();

    Ok(())
}
