use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crdt::Message;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Text synchronization methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum SyncMethod {
    #[serde(rename = "document.create")]
    CreateDocument { name: String },

    #[serde(rename = "document.open")]
    OpenDocument { document_id: Uuid },

    #[serde(rename = "document.update")]
    UpdateDocument {
        document_id: Uuid,
        content: String,
        timestamp: Timestamp,
        client_id: Uuid,
    },

    #[serde(rename = "document.get")]
    GetDocument { document_id: Uuid },

    #[serde(rename = "document.list")]
    ListDocuments,

    #[serde(rename = "client.register")]
    RegisterClient { client_name: String },
}

/// Response types for different methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SyncResponse {
    DocumentUpdated {
        content: String,
    },
    DocumentContent {
        document_id: Uuid,
        content: String,
        last_modified: Timestamp,
    },
    DocumentList {
        documents: Vec<DocumentInfo>,
    },
    ClientRegistered {
        client_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: Uuid,
    pub name: String,
    pub last_modified: Timestamp,
    pub content_length: usize,
}

/// Notification for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBroadcast {
    pub jsonrpc: String,
    pub method: String,
    pub params: SyncBroadcastParams,
}

impl SyncBroadcast {
    pub fn new(method: String, params: SyncBroadcastParams) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentUpdated {
    pub messages: Vec<Message>,
    pub client_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SyncBroadcastParams {
    #[serde(rename = "document_updated")]
    DocumentUpdated {
        messages: Vec<Message>,
        client_id: String,
    },
    #[serde(rename = "client_connected")]
    ClientConnected {
        client_id: String,
        client_name: String,
    },
    #[serde(rename = "client_disconnected")]
    ClientDisconnected { client_id: String },
}

impl JsonRpcRequest {
    pub fn new(method: String, params: serde_json::Value, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id,
        }
    }
}

impl JsonRpcResponse {
    pub fn success(result: serde_json::Value, id: Option<serde_json::Value>) -> Self {
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
