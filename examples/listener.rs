use sink::rpc::SyncNotificationParams;
use sink::{SyncClient, SyncNotification};
use std::io::{self, Write};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyncClient::new("Listener".to_string());
    println!("Connecting to server...");

    loop {
        println!("Listen");
        client
            .start_listening(move |notification| match notification.params {
                SyncNotificationParams::DocumentUpdated {
                    document_id,
                    content,
                    ..
                } => {
                    if document_id == document_id {
                        println!("Document updated: {}", content);
                    }
                }
                rest => {
                    println!("Received notification: {:?}", rest);
                }
            })
            .await
            .unwrap();

        sleep(Duration::from_secs(1)).await;
    }
}
