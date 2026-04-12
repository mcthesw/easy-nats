/// Events sent from the async backend worker to the UI thread.
#[derive(Debug)]
pub enum BackendEvent {
    // Connection
    ConnectionStatus {
        connection_id: u64,
        status: ConnectionStatusKind,
    },
    // Core Pub/Sub
    MessageReceived {
        connection_id: u64,
        subject: String,
        reply: Option<String>,
        headers: Vec<(String, String)>,
        payload: Vec<u8>,
        timestamp: std::time::SystemTime,
    },
    RequestResponse {
        connection_id: u64,
        payload: Vec<u8>,
        headers: Vec<(String, String)>,
    },
    // Operations
    OperationResult {
        connection_id: u64,
        operation: String,
        data: serde_json::Value,
    },
    // Errors
    Error {
        connection_id: Option<u64>,
        operation: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatusKind {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}
