use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

use crate::*;

/// Connected client information
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: String,
    pub name: String,
    pub addr: SocketAddr,
}

/// Shared server state
pub struct ServerState {
    pub crdt: Arc<CRDT>,
    pub clients: RwLock<HashMap<String, ClientInfo>>,
    pub broadcast_tx: broadcast::Sender<SyncBroadcast>,
}

impl ServerState {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            crdt: Arc::new(CRDT::new()),
            clients: RwLock::new(HashMap::new()),
            broadcast_tx,
        }
    }

    pub async fn register_client(&self, client_name: String, addr: SocketAddr) -> String {
        let client_id = nanoid::nanoid!();
        let client_info = ClientInfo {
            id: client_id.clone(),
            name: client_name.clone(),
            addr,
        };

        self.clients
            .write()
            .await
            .insert(client_id.clone(), client_info);

        // Broadcast client connection
        let notification = SyncBroadcast::new(
            SyncBroadcastMethod::ClientConnected,
            SyncBroadcastParams::ClientConnected {
                client_id: client_id.clone(),
                client_name,
            },
        );

        let _ = self.broadcast_tx.send(notification);

        client_id
    }

    pub async fn unregister_client(&self, client_id: String) {
        self.clients.write().await.remove(&client_id);

        // Broadcast client disconnection
        let notification = SyncBroadcast::new(
            SyncBroadcastMethod::ClientDisconnected,
            SyncBroadcastParams::ClientDisconnected { client_id },
        );

        let _ = self.broadcast_tx.send(notification);
    }

    pub async fn broadcast_document_update(&self, messages: Vec<Message>, client_id: String) {
        let notification = SyncBroadcast::new(
            SyncBroadcastMethod::DocumentUpdated,
            SyncBroadcastParams::DocumentUpdated {
                messages,
                client_id,
            },
        );

        let _ = self.broadcast_tx.send(notification);
    }
}

/// Text synchronization server
pub struct SyncServer {
    state: Arc<ServerState>,
}

impl SyncServer {
    pub fn new() -> Self {
        Self {
            state: Arc::new(ServerState::new()),
        }
    }

    pub async fn start(&self, addr: &str) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Text sync server listening on: {}", addr);

        while let Ok((stream, addr)) = listener.accept().await {
            let state = Arc::clone(&self.state);
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, addr, state).await {
                    eprintln!("Error handling connection from {}: {}", addr, e);
                }
            });
        }

        Ok(())
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    state: Arc<ServerState>,
) -> anyhow::Result<()> {
    let ws_stream = accept_async(stream).await?;
    println!("WebSocket connection established: {}", addr);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut client_id: Option<String> = None;
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    loop {
        tokio::select! {
            // Handle incoming messages
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        println!("Received message: {}", text);
                        if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&text) {
                            println!("Received request: {:?}", request);
                            let response = handle_rpc_request(request, &state, &mut client_id, addr).await;
                            if let Ok(response_text) = serde_json::to_string(&response) {
                                if ws_sender.send(WsMessage::Text(response_text.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => break,
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            // Handle broadcasts
            Ok(notification) = broadcast_rx.recv() => {
                if let Ok(notification_text) = serde_json::to_string(&notification) {
                    if ws_sender.send(WsMessage::Text(notification_text.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    // Clean up client registration
    if let Some(id) = client_id {
        state.unregister_client(id).await;
    }

    println!("WebSocket connection closed: {}", addr);
    Ok(())
}

async fn handle_rpc_request(
    request: JsonRpcRequest,
    state: &Arc<ServerState>,
    client_id: &mut Option<String>,
    addr: SocketAddr,
) -> JsonRpcResponse {
    match request.params {
        RpcRequestParams::ClientRegister { client_name } => {
            println!("Registering client: {}", client_name);
            let id = state.register_client(client_name.clone(), addr).await;
            *client_id = Some(id.clone());

            let response = JsonRpcResult::ClientRegistered {
                client_id: id.clone(),
            };
            return JsonRpcResponse::success(response, request.id);
        }
        RpcRequestParams::DocumentUpdate {
            ref messages,
            ref client_id,
        } => {
            println!("Received document update: {:?}", request.params);

            // Merge messages into CRDT
            if let Err(e) = state.crdt.merge(messages.clone()) {
                return JsonRpcResponse::error(JsonRpcError::custom(1, e.to_string()), request.id);
            }

            // Broadcast update to all clients
            state
                .broadcast_document_update(messages.clone(), client_id.clone())
                .await;

            println!("Local state: {}", state.crdt.text().unwrap());

            let response = JsonRpcResult::DocumentUpdated {
                content: "OK".to_string(),
            };
            return JsonRpcResponse::success(response, request.id);
        }
    }
}
