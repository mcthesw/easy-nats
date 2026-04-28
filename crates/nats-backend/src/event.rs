use crate::monitoring::MetricsSnapshot;

#[derive(Debug, Clone)]
pub struct MessageData {
    pub subject: String,
    pub reply: Option<String>,
    pub headers: Vec<(String, String)>,
    pub payload: Vec<u8>,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendOperation {
    Publish,
    Subscribe,
    Unsubscribe,
    Request,
    ListStreams,
    CreateStream,
    UpdateStream,
    DeleteStream,
    PurgeStream,
    GetStreamMessages,
    DeleteStreamMessage,
    ListConsumers,
    CreateConsumer,
    DeleteConsumer,
    UpdateConsumer,
    FetchConsumerMessages,
    ListKvBuckets,
    CreateKvBucket,
    DeleteKvBucket,
    UpdateKvBucket,
    ListKvKeys,
    GetKvEntry,
    PutKvEntry,
    DeleteKvEntry,
    PurgeKvEntry,
    GetKvHistory,
    ListObjectStoreBuckets,
    CreateObjectStoreBucket,
    DeleteObjectStoreBucket,
    ListObjects,
    UploadObject,
    DownloadObject,
    DeleteObject,
    ServerInfo,
    JetStreamAccountInfo,
}

impl BackendOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Publish => "publish",
            Self::Subscribe => "subscribe",
            Self::Unsubscribe => "unsubscribe",
            Self::Request => "request",
            Self::ListStreams => "list_streams",
            Self::CreateStream => "create_stream",
            Self::UpdateStream => "update_stream",
            Self::DeleteStream => "delete_stream",
            Self::PurgeStream => "purge_stream",
            Self::GetStreamMessages => "get_stream_messages",
            Self::DeleteStreamMessage => "delete_stream_message",
            Self::ListConsumers => "list_consumers",
            Self::CreateConsumer => "create_consumer",
            Self::DeleteConsumer => "delete_consumer",
            Self::UpdateConsumer => "update_consumer",
            Self::FetchConsumerMessages => "fetch_consumer_messages",
            Self::ListKvBuckets => "list_kv_buckets",
            Self::CreateKvBucket => "create_kv_bucket",
            Self::DeleteKvBucket => "delete_kv_bucket",
            Self::UpdateKvBucket => "update_kv_bucket",
            Self::ListKvKeys => "list_kv_keys",
            Self::GetKvEntry => "get_kv_entry",
            Self::PutKvEntry => "put_kv_entry",
            Self::DeleteKvEntry => "delete_kv_entry",
            Self::PurgeKvEntry => "purge_kv_entry",
            Self::GetKvHistory => "get_kv_history",
            Self::ListObjectStoreBuckets => "list_object_store_buckets",
            Self::CreateObjectStoreBucket => "create_object_store_bucket",
            Self::DeleteObjectStoreBucket => "delete_object_store_bucket",
            Self::ListObjects => "list_objects",
            Self::UploadObject => "upload_object",
            Self::DownloadObject => "download_object",
            Self::DeleteObject => "delete_object",
            Self::ServerInfo => "server_info",
            Self::JetStreamAccountInfo => "jetstream_account_info",
        }
    }
}

impl std::fmt::Display for BackendOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Events sent from the async backend worker to the UI thread.
#[derive(Debug)]
pub enum BackendEvent {
    // Connection
    ConnectionStatus {
        connection_id: u64,
        status: ConnectionStatusKind,
    },
    // Core Pub/Sub
    MessageBatch {
        connection_id: u64,
        backend_id: u64,
        messages: Vec<MessageData>,
    },
    RequestResponse {
        connection_id: u64,
        backend_id: u64,
        subject: Option<String>,
        payload: Vec<u8>,
        headers: Vec<(String, String)>,
    },
    MetricsSnapshot {
        connection_id: u64,
        snapshot: Box<MetricsSnapshot>,
    },
    // Operations
    OperationResult {
        connection_id: u64,
        operation: BackendOperation,
        data: serde_json::Value,
    },
    // Errors
    Error {
        connection_id: Option<u64>,
        backend_id: Option<u64>,
        operation: BackendOperation,
        message: String,
        data: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatusKind {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}
