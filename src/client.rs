// use crate::rpc::*;
// use chrono::Utc;
// use futures_util::{SinkExt, StreamExt};
// use serde_json::Value;
//
// use tokio::net::TcpStream;
// use tokio::sync::mpsc;
// use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
// use uuid::Uuid;
//
// /// Text synchronization client
// #[derive(Debug)]
// pub struct SyncClient {
//     client_id: Option<Uuid>,
//     client_name: String,
//     ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
//     request_id_counter: u64,
// }
//
// impl SyncClient {
//     pub fn new(client_name: String) -> Self {
//         Self {
//             client_id: None,
//             client_name,
//             ws_stream: None,
//             request_id_counter: 0,
//         }
//     }
//
//     /// Connect to the synchronization server
//     pub async fn connect(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
//         let (ws_stream, _) = connect_async(url).await?;
//         self.ws_stream = Some(ws_stream);
//
//         // Register with the server
//         self.register().await?;
//
//         println!("Connected to sync server at {}", url);
//         Ok(())
//     }
//
//     /// Register this client with the server
//     async fn register(&mut self) -> Result<(), Box<dyn std::error::Error>> {
//         let mut params = serde_json::Map::new();
//         params.insert(
//             "client_name".to_string(),
//             Value::String(self.client_name.clone()),
//         );
//
//         let response = self
//             .send_request("client.register", Value::Object(params))
//             .await?;
//
//         if let Some(result) = response.result {
//             if let Ok(sync_response) = serde_json::from_value::<SyncResponse>(result) {
//                 if let SyncResponse::ClientRegistered { client_id } = sync_response {
//                     self.client_id = Some(client_id);
//                     println!("Registered with client ID: {}", client_id);
//                     return Ok(());
//                 }
//             }
//         }
//
//         Err("Failed to register with server".into())
//     }
//
//     /// Create a new document
//     pub async fn create_document(
//         &mut self,
//         name: String,
//     ) -> Result<Uuid, Box<dyn std::error::Error>> {
//         let mut params = serde_json::Map::new();
//         params.insert("name".to_string(), Value::String(name));
//
//         let response = self
//             .send_request("document.create", Value::Object(params))
//             .await?;
//
//         if let Some(result) = response.result {
//             if let Ok(sync_response) = serde_json::from_value::<SyncResponse>(result) {
//                 if let SyncResponse::DocumentCreated { document_id, .. } = sync_response {
//                     return Ok(document_id);
//                 }
//             }
//         }
//
//         Err("Failed to create document".into())
//     }
//
//     /// Get document content
//     pub async fn get_document(
//         &mut self,
//         document_id: Uuid,
//     ) -> Result<(String, chrono::DateTime<Utc>), Box<dyn std::error::Error>> {
//         let mut params = serde_json::Map::new();
//         params.insert(
//             "document_id".to_string(),
//             Value::String(document_id.to_string()),
//         );
//
//         let response = self
//             .send_request("document.get", Value::Object(params))
//             .await?;
//
//         if let Some(result) = response.result {
//             if let Ok(sync_response) = serde_json::from_value::<SyncResponse>(result) {
//                 if let SyncResponse::DocumentContent {
//                     content,
//                     last_modified,
//                     ..
//                 } = sync_response
//                 {
//                     return Ok((content, last_modified));
//                 }
//             }
//         }
//
//         Err("Failed to get document".into())
//     }
//
//     /// Update document content
//     pub async fn update_document(
//         &mut self,
//         document_id: Uuid,
//         content: String,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         if self.client_id.is_none() {
//             return Err("Client not registered".into());
//         }
//
//         let mut params = serde_json::Map::new();
//         params.insert(
//             "document_id".to_string(),
//             Value::String(document_id.to_string()),
//         );
//         params.insert("content".to_string(), Value::String(content));
//         params.insert(
//             "timestamp".to_string(),
//             Value::String(Utc::now().to_rfc3339()),
//         );
//         params.insert(
//             "client_id".to_string(),
//             Value::String(self.client_id.unwrap().to_string()),
//         );
//
//         let response = self
//             .send_request("document.update", Value::Object(params))
//             .await?;
//
//         if response.error.is_some() {
//             return Err("Failed to update document".into());
//         }
//
//         Ok(())
//     }
//
//     /// List all documents
//     pub async fn list_documents(
//         &mut self,
//     ) -> Result<Vec<DocumentInfo>, Box<dyn std::error::Error>> {
//         let response = self
//             .send_request("document.list", Value::Object(serde_json::Map::new()))
//             .await?;
//
//         if let Some(result) = response.result {
//             if let Ok(sync_response) = serde_json::from_value::<SyncResponse>(result) {
//                 if let SyncResponse::DocumentList { documents } = sync_response {
//                     return Ok(documents);
//                 }
//             }
//         }
//
//         Err("Failed to list documents".into())
//     }
//
//     /// Start listening for notifications from the server
//     pub async fn start_listening<F>(
//         &mut self,
//         mut notification_handler: F,
//     ) -> Result<(), Box<dyn std::error::Error>>
//     where
//         F: FnMut(SyncNotification) + Send + 'static,
//     {
//         if let Some(ws_stream) = self.ws_stream.take() {
//             let (_ws_sender, mut ws_receiver) = ws_stream.split();
//             let (_request_tx, mut _request_rx) = mpsc::unbounded_channel::<(
//                 JsonRpcRequest,
//                 tokio::sync::oneshot::Sender<JsonRpcResponse>,
//             )>();
//
//             // Handle outgoing requests (simplified for demo)
//             let outgoing_task = tokio::spawn(async move {
//                 // In a real implementation, this would handle request queuing
//                 // For now, we'll just keep the connection alive
//                 tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//             });
//
//             // Handle incoming messages
//             let incoming_task = tokio::spawn(async move {
//                 while let Some(msg) = ws_receiver.next().await {
//                     match msg {
//                         Ok(Message::Text(text)) => {
//                             // Try to parse as notification first
//                             if let Ok(notification) =
//                                 serde_json::from_str::<SyncNotification>(&text)
//                             {
//                                 notification_handler(notification);
//                             }
//                             // Otherwise try to parse as response
//                             else if let Ok(_response) =
//                                 serde_json::from_str::<JsonRpcResponse>(&text)
//                             {
//                                 // Handle response - match with pending request
//                                 // This is simplified for the example
//                             }
//                         }
//                         Ok(Message::Close(_)) => break,
//                         Err(e) => {
//                             eprintln!("WebSocket error: {}", e);
//                             break;
//                         }
//                         _ => {}
//                     }
//                 }
//             });
//
//             // Wait for either task to complete
//             tokio::select! {
//                 _ = outgoing_task => {},
//                 _ = incoming_task => {},
//             }
//         }
//
//         Ok(())
//     }
//
//     /// Send a JSON-RPC request and wait for response
//     async fn send_request(
//         &mut self,
//         method: &str,
//         params: Value,
//     ) -> Result<JsonRpcResponse, Box<dyn std::error::Error>> {
//         if self.ws_stream.is_none() {
//             return Err("Not connected to server".into());
//         }
//
//         self.request_id_counter += 1;
//         let request_id = self.request_id_counter;
//
//         let request = JsonRpcRequest::new(
//             method.to_string(),
//             params,
//             Some(Value::Number(serde_json::Number::from(request_id))),
//         );
//
//         let request_text = serde_json::to_string(&request)?;
//
//         // This is a simplified implementation for demonstration
//         // In a production system, you'd want proper async request/response handling
//         if let Some(ws_stream) = self.ws_stream.take() {
//             let (mut sender, mut receiver) = ws_stream.split();
//
//             // Send request
//             sender.send(Message::Text(request_text.into())).await?;
//
//             // Wait for response
//             while let Some(msg) = receiver.next().await {
//                 if let Ok(Message::Text(text)) = msg {
//                     if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&text) {
//                         if response.id == Some(Value::Number(serde_json::Number::from(request_id)))
//                         {
//                             // Reconstruct the stream
//                             self.ws_stream = Some(sender.reunite(receiver)?);
//                             return Ok(response);
//                         }
//                     }
//                 }
//             }
//
//             // If we get here, something went wrong, but try to restore the stream
//             self.ws_stream = Some(sender.reunite(receiver)?);
//         }
//
//         Err("No response received".into())
//     }
//
//     /// Get the client ID
//     pub fn client_id(&self) -> Option<Uuid> {
//         self.client_id
//     }
//
//     /// Get the client name
//     pub fn client_name(&self) -> &str {
//         &self.client_name
//     }
// }
