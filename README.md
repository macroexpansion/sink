# Sink - Text Synchronization Service

A real-time text synchronization service built in Rust using JSON-RPC over WebSockets. Multiple clients can connect and collaboratively edit text documents with automatic conflict resolution.

## Features

- **Real-time synchronization**: Changes are instantly propagated to all connected clients
- **JSON-RPC 2.0 protocol**: Standard, well-defined communication protocol
- **WebSocket transport**: Low-latency, bidirectional communication
- **Conflict resolution**: Simple last-write-wins strategy with timestamps
- **Multiple documents**: Create and manage multiple text documents
- **Client management**: Track connected clients and their activities
- **Concurrent connections**: Support for multiple simultaneous clients
- **Broadcast notifications**: Real-time updates about client connections and document changes

## Implementation Status

✅ **Completed Features:**
- JSON-RPC 2.0 compliant server and client
- WebSocket-based real-time communication
- Client registration and management
- Document creation and retrieval
- Document content updates with timestamps
- Broadcast notifications for client events
- Multiple concurrent client support
- Basic conflict resolution (last-write-wins)
- Comprehensive test examples

🔧 **Working Examples:**
- `test_connection`: Basic connection and document creation test
- `sync_demo`: Multi-client synchronization demonstration
- `simple_client`: Interactive client for manual testing
- Command-line client and server applications

## Architecture

### Server Components
- **SyncServer**: Main server that handles WebSocket connections
- **DocumentStore**: Manages document state and persistence
- **ServerState**: Shared state between all client connections
- **JSON-RPC Handler**: Processes client requests and sends responses

### Client Components
- **SyncClient**: Client library for connecting to the server
- **Request/Response handling**: Async JSON-RPC communication
- **Notification system**: Real-time updates from server

## Quick Start

### 1. Start the Server

```bash
# Start server on default port 8080
cargo run server

# Or specify a custom port
cargo run server 3000
```

### 2. Test Basic Functionality

```bash
# Test basic connection and document creation
cargo run --example test_connection

# Run a comprehensive synchronization demo
cargo run --example sync_demo
```

### 3. Connect Multiple Clients

```bash
# Start a client with a name
cargo run client "Alice"

# Connect to a custom server URL
cargo run client "Bob" "ws://127.0.0.1:3000"
```

### 4. Use the Interactive Example

```bash
# Run the interactive client example
cargo run --example simple_client
```

## JSON-RPC API

### Client Registration

```json
{
  "jsonrpc": "2.0",
  "method": "client.register",
  "params": {
    "client_name": "Alice"
  },
  "id": 1
}
```

### Document Operations

#### Create Document
```json
{
  "jsonrpc": "2.0",
  "method": "document.create",
  "params": {
    "name": "my-document.txt"
  },
  "id": 2
}
```

#### Update Document
```json
{
  "jsonrpc": "2.0",
  "method": "document.update",
  "params": {
    "document_id": "550e8400-e29b-41d4-a716-446655440000",
    "content": "Hello, world!",
    "timestamp": "2023-12-07T10:30:00Z",
    "client_id": "client-uuid"
  },
  "id": 3
}
```

#### Get Document
```json
{
  "jsonrpc": "2.0",
  "method": "document.get",
  "params": {
    "document_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "id": 4
}
```

#### List Documents
```json
{
  "jsonrpc": "2.0",
  "method": "document.list",
  "params": {},
  "id": 5
}
```

### Real-time Notifications

The server sends notifications for real-time updates:

#### Document Updated
```json
{
  "jsonrpc": "2.0",
  "method": "document_updated",
  "params": {
    "type": "document_updated",
    "document_id": "550e8400-e29b-41d4-a716-446655440000",
    "content": "Updated content",
    "timestamp": "2023-12-07T10:30:00Z",
    "client_id": "client-uuid"
  }
}
```

#### Client Connected/Disconnected
```json
{
  "jsonrpc": "2.0",
  "method": "client_connected",
  "params": {
    "type": "client_connected",
    "client_id": "client-uuid",
    "client_name": "Alice"
  }
}
```

## Usage Examples

### Basic Client Usage

```rust
use sink::SyncClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and connect client
    let mut client = SyncClient::new("Alice".to_string());
    client.connect("ws://127.0.0.1:8080").await?;
    
    // Create a document
    let doc_id = client.create_document("shared.txt".to_string()).await?;
    
    // Update document content
    client.update_document(doc_id, "Hello from Alice!".to_string()).await?;
    
    // Get current content
    let (content, last_modified) = client.get_document(doc_id).await?;
    println!("Content: {}", content);
    
    Ok(())
}
```

### Listening for Real-time Updates

```rust
// Start listening for notifications
client.start_listening(|notification| {
    match notification.params {
        SyncNotificationParams::DocumentUpdated { document_id, content, .. } => {
            println!("Document {} updated: {}", document_id, content);
        }
        SyncNotificationParams::ClientConnected { client_name, .. } => {
            println!("Client {} connected", client_name);
        }
        SyncNotificationParams::ClientDisconnected { client_id } => {
            println!("Client {} disconnected", client_id);
        }
    }
}).await?;
```

## Testing

Run the test suite:

```bash
cargo test
```

## Dependencies

- **tokio**: Async runtime
- **serde**: Serialization framework
- **serde_json**: JSON support
- **tokio-tungstenite**: WebSocket implementation
- **uuid**: Unique identifier generation
- **chrono**: Date and time handling
- **futures-util**: Async utilities

## Conflict Resolution

The system uses a simple "last-write-wins" conflict resolution strategy:

1. Each update includes a timestamp
2. The server compares timestamps when receiving updates
3. Updates with newer timestamps overwrite older ones
4. All clients receive the final state after conflict resolution

## Limitations and Future Improvements

- **Persistence**: Documents are currently stored in memory only
- **Authentication**: No authentication or authorization system
- **Advanced Conflict Resolution**: Could implement operational transforms or CRDTs
- **Scalability**: Single-server architecture, could be distributed
- **Document History**: No version history or undo functionality

## License

This project is open source and available under the MIT License.
