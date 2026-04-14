use crate::connection::ConnectionConfig;

/// Commands sent from the UI thread to the async backend worker.
#[derive(Debug)]
pub enum BackendCommand {
    // Connection
    Connect {
        config: ConnectionConfig,
    },
    Disconnect {
        id: u64,
    },
    // Core Pub/Sub
    Publish {
        connection_id: u64,
        subject: String,
        payload: Vec<u8>,
        headers: Option<Vec<(String, String)>>,
    },
    Subscribe {
        connection_id: u64,
        subscriber_id: u32,
        subject: String,
    },
    Unsubscribe {
        connection_id: u64,
        subscriber_id: u32,
        subject: String,
    },
    Request {
        connection_id: u64,
        subject: String,
        payload: Vec<u8>,
        headers: Option<Vec<(String, String)>>,
        timeout_ms: u64,
    },
    // JetStream
    ListStreams {
        connection_id: u64,
    },
    CreateStream {
        connection_id: u64,
        config: serde_json::Value,
    },
    UpdateStream {
        connection_id: u64,
        config: serde_json::Value,
    },
    DeleteStream {
        connection_id: u64,
        name: String,
    },
    PurgeStream {
        connection_id: u64,
        name: String,
        filter: Option<String>,
    },
    GetStreamMessages {
        connection_id: u64,
        stream: String,
        start_sequence: Option<u64>,
        subject_filter: Option<String>,
        batch_size: u64,
    },
    DeleteStreamMessage {
        connection_id: u64,
        stream: String,
        sequence: u64,
    },
    ListConsumers {
        connection_id: u64,
        stream: String,
    },
    CreateConsumer {
        connection_id: u64,
        stream: String,
        config: serde_json::Value,
    },
    DeleteConsumer {
        connection_id: u64,
        stream: String,
        name: String,
    },
    // KV Store
    ListKvBuckets {
        connection_id: u64,
    },
    CreateKvBucket {
        connection_id: u64,
        config: serde_json::Value,
    },
    DeleteKvBucket {
        connection_id: u64,
        bucket: String,
    },
    ListKvKeys {
        connection_id: u64,
        bucket: String,
    },
    GetKvEntry {
        connection_id: u64,
        bucket: String,
        key: String,
    },
    PutKvEntry {
        connection_id: u64,
        bucket: String,
        key: String,
        value: Vec<u8>,
    },
    DeleteKvEntry {
        connection_id: u64,
        bucket: String,
        key: String,
    },
    PurgeKvEntry {
        connection_id: u64,
        bucket: String,
        key: String,
    },
    GetKvHistory {
        connection_id: u64,
        bucket: String,
        key: String,
    },
    // Object Store
    ListObjectStoreBuckets {
        connection_id: u64,
    },
    CreateObjectStoreBucket {
        connection_id: u64,
        config: serde_json::Value,
    },
    DeleteObjectStoreBucket {
        connection_id: u64,
        bucket: String,
    },
    ListObjects {
        connection_id: u64,
        bucket: String,
    },
    UploadObject {
        connection_id: u64,
        bucket: String,
        name: String,
        data: Vec<u8>,
    },
    DownloadObject {
        connection_id: u64,
        bucket: String,
        name: String,
    },
    DeleteObject {
        connection_id: u64,
        bucket: String,
        name: String,
    },
}
