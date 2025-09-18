use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async("ws://127.0.0.1:8080").await?;
    println!("✓ Connected to server");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send registration request
    let register_request = json!({
        "jsonrpc": "2.0",
        "method": "client.register",
        "params": {
            "client_name": "TestClient"
        },
        "id": 1
    });

    let request_text = serde_json::to_string(&register_request)?;
    ws_sender.send(Message::Text(request_text.into())).await?;
    println!("✓ Sent registration request");

    let register_request = json!({
        "jsonrpc": "2.0",
        "method": "client.register",
        "params": {
            "client_name": "TestClient"
        },
        "id": 2
    });

    // Wait for response
    if let Some(msg) = ws_receiver.next().await {
        match msg? {
            Message::Text(text) => {
                println!("✓ Received response: {}", text);

                // Parse the response
                if let Ok(response) = serde_json::from_str::<Value>(&text) {
                    if let Some(result) = response.get("result") {
                        if let Some(client_id) = result.get("client_id") {
                            println!("✓ Successfully registered with client ID: {}", client_id);
                        }
                    }
                }
            }
            _ => println!("Received non-text message"),
        }
    }

    while true {
        if let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    println!("✓ Received message: {}", text);
                    // Check if this is the document creation response
                    if let Ok(response) = serde_json::from_str::<Value>(&text) {
                        println!("{response:?}");
                    }
                }
                _ => println!("Received non-text message"),
            }
        } else {
            break;
        }
    }

    Ok(())
}
