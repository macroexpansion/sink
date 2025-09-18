// use sink::{SyncClient, SyncNotification, SyncServer};
// use std::env;
//
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let args: Vec<String> = env::args().collect();
//
//     if args.len() < 2 {
//         println!("Usage:");
//         println!(
//             "  {} server [port]           - Start the sync server",
//             args[0]
//         );
//         println!("  {} client <name> [url]     - Start a client", args[0]);
//         return Ok(());
//     }
//
//     match args[1].as_str() {
//         "server" => {
//             let port = args.get(2).unwrap_or(&"8080".to_string()).clone();
//             let addr = format!("127.0.0.1:{}", port);
//
//             println!("Starting text synchronization server on {}", addr);
//             let server = SyncServer::new();
//             server.start(&addr).await?;
//         }
//
//         "client" => {
//             if args.len() < 3 {
//                 println!("Client name is required");
//                 return Ok(());
//             }
//
//             let client_name = args[2].clone();
//             let url = args
//                 .get(3)
//                 .unwrap_or(&"ws://127.0.0.1:8080".to_string())
//                 .clone();
//
//             println!("Starting client '{}' connecting to {}", client_name, url);
//
//             let mut client = SyncClient::new(client_name);
//             client.connect(&url).await?;
//
//             // Example usage
//             println!("Creating a test document...");
//             let doc_id = client.create_document("test.txt".to_string()).await?;
//             println!("Created document with ID: {}", doc_id);
//
//             println!("Updating document content...");
//             client
//                 .update_document(doc_id, "Hello, world!".to_string())
//                 .await?;
//
//             println!("Getting document content...");
//             let (content, last_modified) = client.get_document(doc_id).await?;
//             println!(
//                 "Document content: '{}' (last modified: {})",
//                 content, last_modified
//             );
//
//             println!("Listing all documents...");
//             let documents = client.list_documents().await?;
//             for doc in documents {
//                 println!(
//                     "  - {} (ID: {}, {} bytes)",
//                     doc.name, doc.id, doc.content_length
//                 );
//             }
//
//             // Start listening for notifications
//             println!("Listening for real-time updates...");
//             client
//                 .start_listening(|notification: SyncNotification| match notification.params {
//                     sink::SyncNotificationParams::DocumentUpdated {
//                         document_id,
//                         content,
//                         timestamp,
//                         client_id,
//                     } => {
//                         println!(
//                             "Document {} updated by client {}: '{}' at {}",
//                             document_id, client_id, content, timestamp
//                         );
//                     }
//                     sink::SyncNotificationParams::ClientConnected {
//                         client_id,
//                         client_name,
//                     } => {
//                         println!("Client connected: {} ({})", client_name, client_id);
//                     }
//                     sink::SyncNotificationParams::ClientDisconnected { client_id } => {
//                         println!("Client disconnected: {}", client_id);
//                     }
//                 })
//                 .await?;
//         }
//
//         _ => {
//             println!("Unknown command: {}", args[1]);
//             println!("Use 'server' or 'client'");
//         }
//     }
//
//     Ok(())
// }
