// use crate::document::DocumentStore;
// use crate::rpc::*;
// use chrono::Utc;
// use futures_util::{SinkExt, StreamExt};
// use serde_json::Value;
// use std::collections::HashMap;
// use std::net::SocketAddr;
// use std::sync::Arc;
// use tokio::net::{TcpListener, TcpStream};
// use tokio::sync::{broadcast, RwLock};
// use tokio_tungstenite::{accept_async, tungstenite::Message};
// use uuid::Uuid;
//
// /// Connected client information
// #[derive(Debug, Clone)]
// pub struct ClientInfo {
//     pub id: Uuid,
//     pub name: String,
//     pub addr: SocketAddr,
// }
//
// /// Shared server state
// #[derive(Debug)]
// pub struct ServerState {
//     pub documents: RwLock<DocumentStore>,
//     pub clients: RwLock<HashMap<Uuid, ClientInfo>>,
//     pub broadcast_tx: broadcast::Sender<SyncNotification>,
// }
//
// impl ServerState {
//     pub fn new() -> Self {
//         let (broadcast_tx, _) = broadcast::channel(1000);
//         Self {
//             documents: RwLock::new(DocumentStore::new()),
//             clients: RwLock::new(HashMap::new()),
//             broadcast_tx,
//         }
//     }
//
//     pub async fn register_client(&self, client_name: String, addr: SocketAddr) -> Uuid {
//         let client_id = Uuid::new_v4();
//         let client_info = ClientInfo {
//             id: client_id,
//             name: client_name.clone(),
//             addr,
//         };
//
//         self.clients.write().await.insert(client_id, client_info);
//
//         // Broadcast client connection
//         let notification = SyncNotification {
//             jsonrpc: "2.0".to_string(),
//             method: "client_connected".to_string(),
//             params: SyncNotificationParams::ClientConnected {
//                 client_id,
//                 client_name,
//             },
//         };
//
//         let _ = self.broadcast_tx.send(notification);
//         client_id
//     }
//
//     pub async fn unregister_client(&self, client_id: Uuid) {
//         self.clients.write().await.remove(&client_id);
//
//         // Broadcast client disconnection
//         let notification = SyncNotification {
//             jsonrpc: "2.0".to_string(),
//             method: "client_disconnected".to_string(),
//             params: SyncNotificationParams::ClientDisconnected { client_id },
//         };
//
//         let _ = self.broadcast_tx.send(notification);
//     }
//
//     pub async fn broadcast_document_update(
//         &self,
//         document_id: Uuid,
//         content: String,
//         client_id: Uuid,
//     ) {
//         let notification = SyncNotification {
//             jsonrpc: "2.0".to_string(),
//             method: "document_updated".to_string(),
//             params: SyncNotificationParams::DocumentUpdated {
//                 document_id,
//                 content,
//                 timestamp: Utc::now(),
//                 client_id,
//             },
//         };
//
//         let _ = self.broadcast_tx.send(notification);
//     }
// }
//
// /// Text synchronization server
// pub struct SyncServer {
//     state: Arc<ServerState>,
// }
//
// impl SyncServer {
//     pub fn new() -> Self {
//         Self {
//             state: Arc::new(ServerState::new()),
//         }
//     }
//
//     pub async fn start(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
//         let listener = TcpListener::bind(addr).await?;
//         println!("Text sync server listening on: {}", addr);
//
//         while let Ok((stream, addr)) = listener.accept().await {
//             let state = Arc::clone(&self.state);
//             tokio::spawn(async move {
//                 if let Err(e) = handle_connection(stream, addr, state).await {
//                     eprintln!("Error handling connection from {}: {}", addr, e);
//                 }
//             });
//         }
//
//         Ok(())
//     }
// }
//
// async fn handle_connection(
//     stream: TcpStream,
//     addr: SocketAddr,
//     state: Arc<ServerState>,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let ws_stream = accept_async(stream).await?;
//     println!("WebSocket connection established: {}", addr);
//
//     let (mut ws_sender, mut ws_receiver) = ws_stream.split();
//     let mut client_id: Option<Uuid> = None;
//     let mut broadcast_rx = state.broadcast_tx.subscribe();
//
//     loop {
//         tokio::select! {
//             // Handle incoming messages
//             msg = ws_receiver.next() => {
//                 match msg {
//                     Some(Ok(Message::Text(text))) => {
//                         if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&text) {
//                             let response = handle_rpc_request(request, &state, &mut client_id, addr).await;
//                             if let Ok(response_text) = serde_json::to_string(&response) {
//                                 if ws_sender.send(Message::Text(response_text.into())).await.is_err() {
//                                     break;
//                                 }
//                             }
//                         }
//                     }
//                     Some(Ok(Message::Close(_))) => break,
//                     Some(Err(e)) => {
//                         eprintln!("WebSocket error: {}", e);
//                         break;
//                     }
//                     None => break,
//                     _ => {}
//                 }
//             }
//             // Handle broadcasts
//             Ok(notification) = broadcast_rx.recv() => {
//                 if let Ok(notification_text) = serde_json::to_string(&notification) {
//                     if ws_sender.send(Message::Text(notification_text.into())).await.is_err() {
//                         break;
//                     }
//                 }
//             }
//         }
//     }
//
//     // Clean up client registration
//     if let Some(id) = client_id {
//         state.unregister_client(id).await;
//     }
//
//     println!("WebSocket connection closed: {}", addr);
//     Ok(())
// }
//
// async fn handle_rpc_request(
//     request: JsonRpcRequest,
//     state: &Arc<ServerState>,
//     client_id: &mut Option<Uuid>,
//     addr: SocketAddr,
// ) -> JsonRpcResponse {
//     match request.method.as_str() {
//         "client.register" => {
//             if let Ok(params) =
//                 serde_json::from_value::<serde_json::Map<String, Value>>(request.params)
//             {
//                 if let Some(Value::String(client_name)) = params.get("client_name") {
//                     let id = state.register_client(client_name.clone(), addr).await;
//                     *client_id = Some(id);
//
//                     let response = SyncResponse::ClientRegistered { client_id: id };
//                     return JsonRpcResponse::success(
//                         serde_json::to_value(response).unwrap(),
//                         request.id,
//                     );
//                 }
//             }
//             JsonRpcResponse::error(JsonRpcError::invalid_params(), request.id)
//         }
//
//         "document.create" => {
//             if let Ok(params) =
//                 serde_json::from_value::<serde_json::Map<String, Value>>(request.params)
//             {
//                 if let Some(Value::String(name)) = params.get("name") {
//                     let mut documents = state.documents.write().await;
//                     let document = documents.create_document(name.clone());
//
//                     let response = SyncResponse::DocumentCreated {
//                         document_id: document.id,
//                         name: document.name.clone(),
//                     };
//                     return JsonRpcResponse::success(
//                         serde_json::to_value(response).unwrap(),
//                         request.id,
//                     );
//                 }
//             }
//             JsonRpcResponse::error(JsonRpcError::invalid_params(), request.id)
//         }
//
//         "document.get" => {
//             if let Ok(params) =
//                 serde_json::from_value::<serde_json::Map<String, Value>>(request.params)
//             {
//                 if let Some(Value::String(doc_id_str)) = params.get("document_id") {
//                     if let Ok(doc_id) = Uuid::parse_str(doc_id_str) {
//                         let documents = state.documents.read().await;
//                         if let Some(document) = documents.get_document(&doc_id) {
//                             let response = SyncResponse::DocumentContent {
//                                 document_id: document.id,
//                                 content: document.content.clone(),
//                                 last_modified: document.last_modified,
//                             };
//                             return JsonRpcResponse::success(
//                                 serde_json::to_value(response).unwrap(),
//                                 request.id,
//                             );
//                         }
//                     }
//                 }
//             }
//             JsonRpcResponse::error(
//                 JsonRpcError::custom(-1, "Document not found".to_string()),
//                 request.id,
//             )
//         }
//
//         "document.update" => {
//             if let Some(current_client_id) = client_id {
//                 if let Ok(params) =
//                     serde_json::from_value::<serde_json::Map<String, Value>>(request.params)
//                 {
//                     if let (
//                         Some(Value::String(doc_id_str)),
//                         Some(Value::String(content)),
//                         Some(Value::String(timestamp_str)),
//                     ) = (
//                         params.get("document_id"),
//                         params.get("content"),
//                         params.get("timestamp"),
//                     ) {
//                         if let (Ok(doc_id), Ok(timestamp)) = (
//                             Uuid::parse_str(doc_id_str),
//                             chrono::DateTime::parse_from_rfc3339(timestamp_str),
//                         ) {
//                             let mut documents = state.documents.write().await;
//                             if let Some(document) = documents.update_document(
//                                 &doc_id,
//                                 content.clone(),
//                                 *current_client_id,
//                                 timestamp.with_timezone(&Utc),
//                             ) {
//                                 // Broadcast the update to all clients
//                                 state
//                                     .broadcast_document_update(
//                                         doc_id,
//                                         content.clone(),
//                                         *current_client_id,
//                                     )
//                                     .await;
//
//                                 let response = SyncResponse::DocumentUpdated {
//                                     document_id: document.id,
//                                     content: document.content.clone(),
//                                     timestamp: document.last_modified,
//                                 };
//                                 return JsonRpcResponse::success(
//                                     serde_json::to_value(response).unwrap(),
//                                     request.id,
//                                 );
//                             }
//                         }
//                     }
//                 }
//             }
//             JsonRpcResponse::error(JsonRpcError::invalid_params(), request.id)
//         }
//
//         "document.list" => {
//             let documents = state.documents.read().await;
//             let doc_list = documents.list_documents();
//
//             let response = SyncResponse::DocumentList {
//                 documents: doc_list,
//             };
//             JsonRpcResponse::success(serde_json::to_value(response).unwrap(), request.id)
//         }
//
//         _ => JsonRpcResponse::error(JsonRpcError::method_not_found(), request.id),
//     }
// }
