use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Text Synchronization Demo ===");

    // Connect first client
    let (ws_stream1, _) = connect_async("ws://127.0.0.1:8080").await?;
    println!("✓ Client 1 connected");
    let (mut sender1, mut receiver1) = ws_stream1.split();

    // Connect second client
    let (ws_stream2, _) = connect_async("ws://127.0.0.1:8080").await?;
    println!("✓ Client 2 connected");
    let (mut sender2, mut receiver2) = ws_stream2.split();

    // Register Client 1
    let register1 = json!({
        "jsonrpc": "2.0",
        "method": "client.register",
        "params": { "client_name": "Alice" },
        "id": 1
    });
    sender1
        .send(Message::Text(serde_json::to_string(&register1)?.into()))
        .await?;

    // Register Client 2
    let register2 = json!({
        "jsonrpc": "2.0",
        "method": "client.register",
        "params": { "client_name": "Bob" },
        "id": 1
    });
    sender2
        .send(Message::Text(serde_json::to_string(&register2)?.into()))
        .await?;

    // Get registration responses
    let mut client1_id = None;
    let mut client2_id = None;
    let mut registrations_complete = 0;

    // Process registration responses and notifications
    while registrations_complete < 2 {
        tokio::select! {
            msg1 = receiver1.next() => {
                if let Some(Ok(Message::Text(text))) = msg1 {
                    println!("Alice received: {}", text);
                    if let Ok(response) = serde_json::from_str::<Value>(&text) {
                        if response.get("id") == Some(&json!(1)) {
                            if let Some(result) = response.get("result") {
                                if let Some(id) = result.get("client_id") {
                                    client1_id = Some(id.as_str().unwrap().to_string());
                                    println!("✓ Alice registered with ID: {}", id);
                                    registrations_complete += 1;
                                }
                            }
                        }
                    }
                }
            }
            msg2 = receiver2.next() => {
                if let Some(Ok(Message::Text(text))) = msg2 {
                    println!("Bob received: {}", text);
                    if let Ok(response) = serde_json::from_str::<Value>(&text) {
                        if response.get("id") == Some(&json!(1)) {
                            if let Some(result) = response.get("result") {
                                if let Some(id) = result.get("client_id") {
                                    client2_id = Some(id.as_str().unwrap().to_string());
                                    println!("✓ Bob registered with ID: {}", id);
                                    registrations_complete += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Client 1 creates a document
    let create_doc = json!({
        "jsonrpc": "2.0",
        "method": "document.create",
        "params": { "name": "shared_document.txt" },
        "id": 2
    });
    sender1
        .send(Message::Text(serde_json::to_string(&create_doc)?.into()))
        .await?;

    // Get document creation response and notifications
    let mut document_id = None;
    let mut messages_received = 0;

    // Process messages until we get the document creation response
    while messages_received < 4 {
        // Expect: 2 client_connected notifications + 1 doc creation response
        tokio::select! {
            msg1 = receiver1.next() => {
                if let Some(Ok(Message::Text(text))) = msg1 {
                    println!("Alice received: {}", text);
                    if let Ok(response) = serde_json::from_str::<Value>(&text) {
                        if response.get("id") == Some(&json!(2)) {
                            if let Some(result) = response.get("result") {
                                if let Some(doc_id) = result.get("document_id") {
                                    document_id = Some(doc_id.as_str().unwrap().to_string());
                                    println!("✓ Document created with ID: {}", doc_id);
                                }
                            }
                        }
                    }
                    messages_received += 1;
                }
            }
            msg2 = receiver2.next() => {
                if let Some(Ok(Message::Text(text))) = msg2 {
                    println!("Bob received: {}", text);
                    messages_received += 1;
                }
            }
        }
    }

    if let Some(doc_id) = document_id {
        println!("\n=== Testing Document Updates ===");

        // Alice updates the document
        let update_doc = json!({
            "jsonrpc": "2.0",
            "method": "document.update",
            "params": {
                "document_id": doc_id,
                "content": "Hello from Alice!",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "client_id": client1_id.as_ref().unwrap()
            },
            "id": 3
        });
        sender1
            .send(Message::Text(serde_json::to_string(&update_doc)?.into()))
            .await?;

        // Wait for update responses and notifications
        let mut updates_received = 0;
        while updates_received < 2 {
            // Expect response + notification
            tokio::select! {
                msg1 = receiver1.next() => {
                    if let Some(Ok(Message::Text(text))) = msg1 {
                        println!("Alice received: {}", text);
                        updates_received += 1;
                    }
                }
                msg2 = receiver2.next() => {
                    if let Some(Ok(Message::Text(text))) = msg2 {
                        println!("Bob received: {}", text);
                        updates_received += 1;
                    }
                }
            }
        }

        // Bob updates the document
        let update_doc2 = json!({
            "jsonrpc": "2.0",
            "method": "document.update",
            "params": {
                "document_id": doc_id,
                "content": "Hello from Bob! Alice said: Hello from Alice!",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "client_id": client2_id.as_ref().unwrap()
            },
            "id": 3
        });
        sender2
            .send(Message::Text(serde_json::to_string(&update_doc2)?.into()))
            .await?;

        // Wait for final updates
        let mut final_updates = 0;
        while final_updates < 2 {
            tokio::select! {
                msg1 = receiver1.next() => {
                    if let Some(Ok(Message::Text(text))) = msg1 {
                        println!("Alice received: {}", text);
                        final_updates += 1;
                    }
                }
                msg2 = receiver2.next() => {
                    if let Some(Ok(Message::Text(text))) = msg2 {
                        println!("Bob received: {}", text);
                        final_updates += 1;
                    }
                }
            }
        }

        // Get final document state
        let get_doc = json!({
            "jsonrpc": "2.0",
            "method": "document.get",
            "params": { "document_id": doc_id },
            "id": 4
        });
        sender1
            .send(Message::Text(serde_json::to_string(&get_doc)?.into()))
            .await?;

        if let Some(Ok(Message::Text(text))) = receiver1.next().await {
            println!("\nFinal document state: {}", text);
        }
    }

    println!("\n✓ Synchronization demo completed successfully!");
    Ok(())
}
