use crate::cancellation::TaskCancellation;
use crate::connection::ConnectionConfig;
use crate::models::{
    ConsumerConfigInput, KvBucketConfigInput, ObjectStoreBucketConfigInput, StreamConfigInput,
};
use crate::monitoring::ClientStatusQuery;

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
        backend_id: u64,
        subject: String,
        cancel: crate::TaskCancellation,
    },
    Unsubscribe {
        connection_id: u64,
        backend_id: u64,
        subject: String,
    },
    Request {
        connection_id: u64,
        backend_id: u64,
        request_id: u64,
        subject: String,
        payload: Vec<u8>,
        headers: Option<Vec<(String, String)>>,
        timeout_ms: u64,
    },
    Reply {
        connection_id: u64,
        backend_id: u64,
        reply_id: u64,
        reply_to: String,
        payload: Vec<u8>,
        headers: Option<Vec<(String, String)>>,
    },
    // JetStream
    ListStreams {
        connection_id: u64,
    },
    CreateStream {
        connection_id: u64,
        config: StreamConfigInput,
    },
    UpdateStream {
        connection_id: u64,
        config: StreamConfigInput,
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
        start_time: Option<String>,
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
        config: ConsumerConfigInput,
    },
    DeleteConsumer {
        connection_id: u64,
        stream: String,
        name: String,
    },
    UpdateConsumer {
        connection_id: u64,
        stream: String,
        config: ConsumerConfigInput,
    },
    FetchConsumerMessages {
        connection_id: u64,
        stream: String,
        consumer: String,
        batch: usize,
    },
    // KV Store
    ListKvBuckets {
        connection_id: u64,
    },
    CreateKvBucket {
        connection_id: u64,
        config: KvBucketConfigInput,
    },
    DeleteKvBucket {
        connection_id: u64,
        bucket: String,
    },
    UpdateKvBucket {
        connection_id: u64,
        config: KvBucketConfigInput,
    },
    ListKvKeys {
        connection_id: u64,
        bucket: String,
        cancel: TaskCancellation,
        generation: u64,
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
        config: ObjectStoreBucketConfigInput,
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
        file_path: std::path::PathBuf,
    },
    DeleteObject {
        connection_id: u64,
        bucket: String,
        name: String,
    },
    // Server Info
    GetServerInfo {
        connection_id: u64,
    },
    GetJetStreamAccountInfo {
        connection_id: u64,
    },
    // Monitoring
    FetchMetrics {
        connection_id: u64,
        endpoint: String,
    },
    FetchClientStatusPage {
        connection_id: u64,
        endpoint: String,
        query: ClientStatusQuery,
    },
    FetchClientStatusDetail {
        connection_id: u64,
        endpoint: String,
        query: ClientStatusQuery,
    },
}
