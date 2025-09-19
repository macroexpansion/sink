use std::sync::Arc;

use anyhow::anyhow;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use jiff::Timestamp;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};

use crate::*;

/// Text synchronization client
#[derive(Debug)]
pub struct SyncClient {
    state: Arc<CRDT>,
    client_id: Option<String>,
    client_name: String,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl SyncClient {
    pub fn new(client_name: String) -> Self {
        Self {
            state: Arc::new(CRDT::new()),
            client_id: None,
            client_name,
            ws_stream: None,
        }
    }

    /// Connect to the synchronization server
    pub async fn connect(&mut self, url: &str) -> anyhow::Result<()> {
        let (ws_stream, _) = connect_async(url).await?;
        self.ws_stream = Some(ws_stream);

        // Register with the server
        self.register().await?;

        println!("Connected to sync server at {}", url);
        Ok(())
    }

    /// Register this client with the server
    async fn register(&mut self) -> anyhow::Result<()> {
        let response = self
            .send_request(RpcRequestParams::ClientRegister {
                client_name: self.client_name.clone(),
            })
            .await?;

        if let Some(result) = response.result {
            if let JsonRpcResult::ClientRegistered { client_id } = result {
                self.client_id = Some(client_id.clone());
                println!("Registered with client ID: {}", client_id);
                return Ok(());
            }
        }

        Err(anyhow!("Failed to register with server"))
    }

    /// Create a new message
    pub async fn create_message(
        state: Arc<CRDT>,
        sender: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
        content: String,
        client_id: String,
    ) -> anyhow::Result<()> {
        let messages = vec![Message {
            id: nanoid::nanoid!(),
            operation: Operation::Add(content),
            timestamp: Timestamp::now(),
        }];

        state.merge(messages.clone())?;

        Self::send_rpc_request(
            sender,
            RpcRequestParams::DocumentUpdate {
                messages,
                client_id,
            },
        )
        .await?;

        Ok(())
    }

    /// Send a JSON-RPC request over the WebSocketj
    pub async fn send_rpc_request(
        sender: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
        params: RpcRequestParams,
    ) -> anyhow::Result<()> {
        let request = JsonRpcRequest::new(params);

        let request_text = serde_json::to_string(&request)?;
        sender.send(WsMessage::Text(request_text.into())).await?;

        Ok(())
    }

    /// Start listening for notifications from the server
    pub async fn start(&mut self) -> anyhow::Result<()> {
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
                            else if let Ok(response) =
                                serde_json::from_str::<JsonRpcResult>(&text)
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
    async fn send_request(&mut self, params: RpcRequestParams) -> anyhow::Result<JsonRpcResponse> {
        if self.ws_stream.is_none() {
            return Err(anyhow!("Not connected to server"));
        }

        let request = JsonRpcRequest::new(params);

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

        Err(anyhow!("No response received"))
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
