use serde::{Deserialize, Serialize};

use crate::Message;

/// Text synchronization methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcMethod {
    #[serde(rename = "document.update")]
    DocumentUpdate,

    #[serde(rename = "client.register")]
    ClientRegister,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
#[cfg_attr(test, derive(PartialEq))]
pub enum RpcRequestParams {
    #[serde(rename = "document.update")]
    DocumentUpdate {
        messages: Vec<Message>,
        client_id: String,
    },

    #[serde(rename = "client.register")]
    ClientRegister { client_name: String },
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct JsonRpcRequest {
    pub jsonrpc: String,

    #[serde(flatten)]
    pub params: RpcRequestParams,

    pub id: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(params: RpcRequestParams) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            params,
            id: None,
        }
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<JsonRpcResult>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    pub id: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn success(result: JsonRpcResult, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(error: JsonRpcError, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// Response types for different methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResult {
    DocumentUpdated { content: String },
    ClientRegistered { client_id: String },
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: None,
        }
    }

    pub fn method_not_found() -> Self {
        Self {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        }
    }

    pub fn invalid_params() -> Self {
        Self {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        }
    }

    pub fn internal_error() -> Self {
        Self {
            code: -32603,
            message: "Internal error".to_string(),
            data: None,
        }
    }

    pub fn custom(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }
}

/// Notification for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBroadcast {
    pub jsonrpc: String,
    pub method: SyncBroadcastMethod,
    pub params: SyncBroadcastParams,
}

impl SyncBroadcast {
    pub fn new(method: SyncBroadcastMethod, params: SyncBroadcastParams) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncBroadcastMethod {
    #[serde(rename = "document.updated")]
    DocumentUpdated,
    #[serde(rename = "client.connected")]
    ClientConnected,
    #[serde(rename = "client.disconnected")]
    ClientDisconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SyncBroadcastParams {
    #[serde(rename = "document.updated")]
    DocumentUpdated {
        messages: Vec<Message>,
        client_id: String,
    },
    #[serde(rename = "client.connected")]
    ClientConnected {
        client_id: String,
        client_name: String,
    },
    #[serde(rename = "client.disconnected")]
    ClientDisconnected { client_id: String },
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use crate::*;

    use super::*;

    #[test]
    fn test_json_rpc_request() {
        let requests = vec![
            JsonRpcRequest::new(RpcRequestParams::ClientRegister {
                client_name: "Alice".to_string(),
            }),
            JsonRpcRequest::new(RpcRequestParams::DocumentUpdate {
                messages: vec![
                    Message {
                        id: "1".to_string(),
                        operation: Operation::Add("Hello".to_string()),
                        timestamp: Timestamp::now(),
                    },
                    Message {
                        id: "2".to_string(),
                        operation: Operation::Add("World".to_string()),
                        timestamp: Timestamp::now(),
                    },
                ],
                client_id: "Alice".to_string(),
            }),
        ];

        for request in requests {
            let se = serde_json::to_string(&request).unwrap();
            let de: JsonRpcRequest = serde_json::from_str(&se).unwrap();
            assert_eq!(request, de);
        }
    }
}
