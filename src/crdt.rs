use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{anyhow, Result};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{
    accept_async, connect_async, tungstenite::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};

use crate::rpc::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, SyncBroadcast, SyncBroadcastParams, SyncResponse,
};
use crate::DocumentUpdated;

pub type MessageID = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    Add(String),       // add content
    Remove(MessageID), // remove by message ID
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub operation: Operation,
    pub timestamp: Timestamp,
}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.timestamp.cmp(&other.timestamp) {
            Ordering::Equal => self.id.cmp(&other.id),
            other => other,
        }
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Text {
    pub id: String,
    pub data: String,
    pub timestamp: Timestamp,
    pub removed: RefCell<bool>,
}

impl Ord for Text {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.timestamp.cmp(&other.timestamp) {
            Ordering::Equal => self.id.cmp(&other.id),
            other => other,
        }
    }
}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&Message> for Text {
    fn from(value: &Message) -> Self {
        match &value.operation {
            Operation::Add(data) => Self {
                id: value.id.clone(),
                data: data.clone(),
                timestamp: value.timestamp.clone(),
                removed: RefCell::new(false),
            },
            Operation::Remove(_) => unreachable!(),
        }
    }
}

/// RGA-inspired CRDT implementation
#[derive(Debug, Clone)]
pub struct CRDT {
    replica_id: String,
    ops: Arc<Mutex<BTreeSet<Message>>>,
    content: Arc<Mutex<BTreeSet<Text>>>,
}

impl CRDT {
    pub fn new() -> Self {
        Self {
            replica_id: nanoid::nanoid!(),
            ops: Arc::new(Mutex::new(BTreeSet::new())),
            content: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    fn update(
        locked_ops: &mut MutexGuard<'_, BTreeSet<Message>>,
        locked_content: &mut MutexGuard<'_, BTreeSet<Text>>,
        message: Message,
    ) -> Result<()> {
        match &message.operation {
            Operation::Add(_) => {
                locked_content.insert(Text::from(&message));
            }
            Operation::Remove(id) => {
                if let Some(item) = locked_content.iter().find(|p| p.id == *id) {
                    *item.removed.borrow_mut() = true;
                }
            }
        }
        locked_ops.insert(message);

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Message>> {
        let locked_ops = self.ops.lock().map_err(|_| anyhow!("lock error"))?;
        Ok(locked_ops.iter().cloned().collect())
    }

    pub fn merge(&self, mut ops: Vec<Message>) -> Result<()> {
        ops.sort_by(|x, y| x.timestamp.cmp(&y.timestamp));

        let mut locked_ops = self.ops.lock().map_err(|_| anyhow!("lock error"))?;
        let mut locked_content = self.content.lock().map_err(|_| anyhow!("lock error"))?;

        for op in ops {
            Self::update(&mut locked_ops, &mut locked_content, op)?;
        }

        Ok(())
    }

    pub fn text(&self) -> Result<String> {
        let mut text = String::new();
        let locked_ops = self.content.lock().map_err(|_| anyhow!("lock error"))?;
        for op in locked_ops.iter().rev() {
            if !*op.removed.borrow() {
                let element = format!(
                    "\n# id: {id}, timestamp: {timestamp}\n{data}",
                    id = op.id,
                    timestamp = op.timestamp,
                    data = op.data
                );
                text.push_str(&element);
            }
        }
        Ok(text)
    }
}

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
            "client_connected".to_string(),
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
            "client_disconnected".to_string(),
            SyncBroadcastParams::ClientDisconnected { client_id },
        );

        let _ = self.broadcast_tx.send(notification);
    }

    pub async fn broadcast_document_update(&self, messages: Vec<Message>, client_id: String) {
        let notification = SyncBroadcast::new(
            "document_updated".to_string(),
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

    pub async fn start(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
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
) -> Result<(), Box<dyn std::error::Error>> {
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
    match request.method.as_str() {
        "client.register" => {
            if let Ok(params) =
                serde_json::from_value::<serde_json::Map<String, Value>>(request.params)
            {
                if let Some(Value::String(client_name)) = params.get("client_name") {
                    println!("Registering client: {}", client_name);
                    let id = state.register_client(client_name.clone(), addr).await;
                    *client_id = Some(id.clone());

                    let response = SyncResponse::ClientRegistered {
                        client_id: id.clone(),
                    };

                    return JsonRpcResponse::success(
                        serde_json::to_value(response).unwrap(),
                        request.id,
                    );
                }
            }
            JsonRpcResponse::error(JsonRpcError::invalid_params(), request.id)
        }
        "document.update" => {
            if let Ok(params) = serde_json::from_value::<DocumentUpdated>(request.params) {
                println!("Received document update: {:?}", params);

                // Merge messages into CRDT
                state.crdt.merge(params.messages.clone()).unwrap();

                // Broadcast update to all clients
                state
                    .broadcast_document_update(params.messages, params.client_id)
                    .await;

                println!("Local state: {}", state.crdt.text().unwrap());

                let response = SyncResponse::DocumentUpdated {
                    content: "OK".to_string(),
                };
                return JsonRpcResponse::success(
                    serde_json::to_value(response).unwrap(),
                    request.id,
                );
            }
            JsonRpcResponse::error(JsonRpcError::invalid_params(), request.id)
        }
        _ => JsonRpcResponse::error(JsonRpcError::method_not_found(), request.id),
    }
}

/// Text synchronization client
#[derive(Debug)]
pub struct SyncClient {
    state: Arc<CRDT>,
    client_id: Option<String>,
    client_name: String,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    request_id_counter: u64,
}

impl SyncClient {
    pub fn new(client_name: String) -> Self {
        Self {
            state: Arc::new(CRDT::new()),
            client_id: None,
            client_name,
            ws_stream: None,
            request_id_counter: 0,
        }
    }

    /// Connect to the synchronization server
    pub async fn connect(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(url).await?;
        self.ws_stream = Some(ws_stream);

        // Register with the server
        self.register().await?;

        println!("Connected to sync server at {}", url);
        Ok(())
    }

    /// Register this client with the server
    async fn register(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut params = serde_json::Map::new();
        params.insert(
            "client_name".to_string(),
            Value::String(self.client_name.clone()),
        );

        let response = self
            .send_request("client.register", Value::Object(params))
            .await?;

        if let Some(result) = response.result {
            if let Ok(sync_response) = serde_json::from_value::<SyncResponse>(result) {
                if let SyncResponse::ClientRegistered { client_id } = sync_response {
                    self.client_id = Some(client_id.clone());
                    println!("Registered with client ID: {}", client_id);
                    return Ok(());
                }
            }
        }

        Err("Failed to register with server".into())
    }

    /// Create a new message
    pub async fn create_message(
        state: Arc<CRDT>,
        sender: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
        content: String,
        client_id: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let messages = vec![Message {
            id: nanoid::nanoid!(),
            operation: Operation::Add(content),
            timestamp: Timestamp::now(),
        }];

        state.merge(messages.clone())?;

        let params = serde_json::to_value(DocumentUpdated {
            messages,
            client_id,
        })?;
        Self::send_rpc_request(sender, "document.update", params).await?;

        Ok(())
    }

    /// Send a JSON-RPC request over the WebSocketj
    pub async fn send_rpc_request(
        sender: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
        method: &str,
        params: Value,
    ) -> anyhow::Result<()> {
        let request = JsonRpcRequest::new(method.to_string(), params, None);

        let request_text = serde_json::to_string(&request)?;
        sender.send(WsMessage::Text(request_text.into())).await?;

        Ok(())
    }

    /// Start listening for notifications from the server
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ws_stream) = self.ws_stream.take() {
            let (mut ws_sender, mut ws_receiver) = ws_stream.split();
            // let (_request_tx, mut _request_rx) = mpsc::unbounded_channel::<(
            //     JsonRpcRequest,
            //     tokio::sync::oneshot::Sender<JsonRpcResponse>,
            // )>();

            let client_id = self.client_id.clone().unwrap();
            let state = Arc::clone(&self.state);

            let outgoing_task = tokio::spawn({
                let state = Arc::clone(&state);
                let client_id = client_id.clone();
                async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

                        Self::create_message(
                            state.clone(),
                            &mut ws_sender,
                            "Hello".to_string(),
                            client_id.clone(),
                        )
                        .await
                        .unwrap();

                        let text = state.text().unwrap();
                        println!("Local state: {}", text);
                    }
                }
            });

            // Handle incoming messages
            let incoming_task = tokio::spawn(async move {
                while let Some(msg) = ws_receiver.next().await {
                    match msg {
                        Ok(WsMessage::Text(text)) => {
                            // Try to parse as notification first
                            if let Ok(notification) = serde_json::from_str::<SyncBroadcast>(&text) {
                                println!("Received notification: {:?}", notification);
                                match notification.params {
                                    SyncBroadcastParams::DocumentUpdated {
                                        messages,
                                        client_id: sender_id,
                                    } => {
                                        if client_id.clone() != sender_id {
                                            println!("Merging messages...");
                                            Arc::clone(&state).merge(messages).unwrap();
                                            let text = state.text().unwrap();
                                            println!("Local state: {}", text);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            // Otherwise try to parse as response
                            else if let Ok(response) =
                                serde_json::from_str::<JsonRpcResponse>(&text)
                            {
                                println!("Received response: {:?}", response);
                            }
                            // Otherwise try to parse as response
                            else if let Ok(response) = serde_json::from_str::<SyncResponse>(&text)
                            {
                                println!("Received response: {:?}", response);
                            }
                        }
                        Ok(WsMessage::Close(_)) => break,
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            });

            _ = tokio::join!(outgoing_task, incoming_task);
        }

        Ok(())
    }

    /// Send a JSON-RPC request and wait for response
    async fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<JsonRpcResponse, Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("Not connected to server".into());
        }

        let request = JsonRpcRequest::new(method.to_string(), params, None);

        let request_text = serde_json::to_string(&request)?;

        // This is a simplified implementation for demonstration
        // In a production system, you'd want proper async request/response handling
        if let Some(ws_stream) = self.ws_stream.take() {
            let (mut sender, mut receiver) = ws_stream.split();

            // Send request
            sender.send(WsMessage::Text(request_text.into())).await?;

            // Wait for response
            while let Some(msg) = receiver.next().await {
                if let Ok(WsMessage::Text(text)) = msg {
                    if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&text) {
                        // if response.id == Some(Value::Number(serde_json::Number::from(1))) {
                        //     // Reconstruct the stream
                        //     self.ws_stream = Some(sender.reunite(receiver)?);
                        //     return Ok(response);
                        // }
                        self.ws_stream = Some(sender.reunite(receiver)?);
                        return Ok(response);
                    }
                }
            }

            // If we get here, something went wrong, but try to restore the stream
            self.ws_stream = Some(sender.reunite(receiver)?);
        }

        Err("No response received".into())
    }

    /// Get the client ID
    pub fn client_id(&self) -> Option<String> {
        self.client_id.clone()
    }

    /// Get the client name
    pub fn client_name(&self) -> &str {
        &self.client_name
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use jiff::Timestamp;
    use rand::rng;
    use rand::seq::SliceRandom;

    use super::*;

    #[test]
    fn test_crdt() {
        let crdt = CRDT::new();

        let mut ops = vec![
            Message {
                id: String::from("1"),
                operation: Operation::Add(String::from("1")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:01.693605086Z").unwrap(),
            },
            Message {
                id: String::from("2"),
                operation: Operation::Add(String::from("2")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:02.693605086Z").unwrap(),
            },
            Message {
                id: String::from("3"),
                operation: Operation::Add(String::from("3")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:03.693605086Z").unwrap(),
            },
            Message {
                id: String::from("4"),
                operation: Operation::Add(String::from("4")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:04.693605086Z").unwrap(),
            },
            Message {
                id: String::from("5"),
                operation: Operation::Add(String::from("5")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:05.693605086Z").unwrap(),
            },
            Message {
                id: String::from("6"),
                operation: Operation::Add(String::from("6")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:06.693605086Z").unwrap(),
            },
            Message {
                id: String::from("7"),
                operation: Operation::Remove(MessageID::from("5")),
                timestamp: Timestamp::from_str("2025-08-11T15:48:07.693605086Z").unwrap(),
            },
        ];

        let mut rng = rng();
        ops.shuffle(&mut rng);

        crdt.merge(ops).unwrap();

        let text = crdt.text().unwrap();

        assert_eq!(
            text,
            String::from(
                r###"
# id: 1, timestamp: 2025-08-11T15:48:01.693605086Z
1
# id: 2, timestamp: 2025-08-11T15:48:02.693605086Z
2
# id: 3, timestamp: 2025-08-11T15:48:03.693605086Z
3
# id: 4, timestamp: 2025-08-11T15:48:04.693605086Z
4
# id: 6, timestamp: 2025-08-11T15:48:06.693605086Z
6"###
            )
        );
    }
}
