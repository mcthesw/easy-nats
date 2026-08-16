use std::time::Duration;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    #[default]
    File,
    Memory,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StreamRetentionKind {
    #[default]
    Limits,
    Interest,
    WorkQueue,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ConsumerDeliverPolicyKind {
    #[default]
    All,
    Last,
    New,
    ByStartSequence {
        start_sequence: u64,
    },
    ByStartTime {
        start_time: String,
    },
    LastPerSubject,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerAckPolicyKind {
    #[default]
    Explicit,
    All,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamConfigInput {
    pub name: String,
    pub subjects: Vec<String>,
    pub storage: StorageKind,
    pub retention: StreamRetentionKind,
    pub max_messages: Option<i64>,
    pub max_bytes: Option<i64>,
    pub max_age: Option<Duration>,
    pub num_replicas: Option<usize>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerConfigInput {
    pub name: String,
    pub durable_name: Option<String>,
    pub filter_subject: Option<String>,
    pub deliver_policy: ConsumerDeliverPolicyKind,
    pub ack_policy: ConsumerAckPolicyKind,
    pub max_deliver: Option<i64>,
    pub max_ack_pending: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvBucketConfigInput {
    pub bucket: String,
    pub history: i64,
    pub storage: StorageKind,
    pub max_value_size: Option<i32>,
    pub max_bytes: Option<i64>,
    pub max_age: Option<Duration>,
    pub num_replicas: Option<usize>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStoreBucketConfigInput {
    pub bucket: String,
    pub storage: StorageKind,
    pub max_bytes: Option<i64>,
    pub num_replicas: Option<usize>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvBucketInfo {
    pub bucket: String,
    pub stored_history_values: u64,
    pub history_depth: i64,
    pub max_age_secs: u64,
    pub max_age_nanos: u64,
    pub description: String,
    pub storage: String,
    pub bytes: u64,
    pub max_bytes: i64,
    pub max_value_size: i64,
    pub num_replicas: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvKeyBatch {
    pub bucket: String,
    pub keys: Vec<String>,
    pub done: bool,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvEntryInfo {
    pub bucket: String,
    pub key: String,
    pub value: Vec<u8>,
    pub revision: Option<u64>,
    pub delta: Option<u64>,
    pub created: Option<String>,
    pub operation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvHistoryItem {
    pub key: String,
    pub value: Vec<u8>,
    pub revision: u64,
    pub delta: u64,
    pub created: String,
    pub operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStoreBucketInfo {
    pub bucket: String,
    pub description: String,
    pub storage: String,
    pub bytes: u64,
    pub max_bytes: i64,
    pub object_count: u64,
    pub num_replicas: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStoreObjectInfo {
    pub bucket: String,
    pub name: String,
    pub description: String,
    pub size: usize,
    pub chunks: usize,
    pub modified: Option<String>,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStoreDownloadResult {
    pub bucket: String,
    pub name: String,
    pub file_path: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfoSnapshot {
    pub server_id: String,
    pub server_name: String,
    pub version: String,
    pub host: String,
    pub port: u16,
    pub proto: i8,
    pub go: String,
    pub max_payload: usize,
    pub client_id: u64,
    pub auth_required: bool,
    pub tls_required: bool,
    pub connect_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JetStreamAccountLimitsSnapshot {
    pub max_memory: Option<i64>,
    pub max_storage: Option<i64>,
    pub max_streams: Option<i64>,
    pub max_consumers: Option<i64>,
    pub max_ack_pending: i64,
    pub memory_max_stream_bytes: Option<i64>,
    pub storage_max_stream_bytes: Option<i64>,
    pub max_bytes_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JetStreamAccountInfoSnapshot {
    pub memory: u64,
    pub storage: u64,
    pub streams: usize,
    pub consumers: usize,
    pub domain: Option<String>,
    pub limits: JetStreamAccountLimitsSnapshot,
    pub api_total: u64,
    pub api_errors: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamInfo {
    pub name: String,
    pub subjects: Vec<String>,
    pub storage: String,
    pub retention: String,
    pub messages: u64,
    pub bytes: u64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub consumer_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamMessageInfo {
    pub sequence: u64,
    pub subject: String,
    pub payload: Vec<u8>,
    pub headers: Vec<(String, String)>,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerInfo {
    pub name: String,
    pub stream_name: String,
    pub durable_name: Option<String>,
    pub filter_subject: Option<String>,
    pub deliver_policy: ConsumerDeliverPolicyKind,
    pub ack_policy: String,
    pub max_deliver: i64,
    pub max_ack_pending: i64,
    pub description: Option<String>,
    pub deliver_subject: Option<String>,
    pub num_pending: u64,
    pub num_ack_pending: u64,
    pub num_waiting: u64,
    pub num_redelivered: u64,
    pub push_bound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendErrorContext {
    KvEntry {
        bucket: String,
        key: String,
    },
    WorkQueueConsumerPreview {
        stream: String,
        consumer: String,
        reason: String,
    },
}

impl StorageKind {
    pub fn from_display(value: &str) -> Self {
        if value.eq_ignore_ascii_case("memory") {
            Self::Memory
        } else {
            Self::File
        }
    }

    #[cfg(feature = "native")]
    fn into_async_nats(self) -> async_nats::jetstream::stream::StorageType {
        match self {
            Self::File => async_nats::jetstream::stream::StorageType::File,
            Self::Memory => async_nats::jetstream::stream::StorageType::Memory,
        }
    }
}

impl StreamRetentionKind {
    #[cfg(feature = "native")]
    fn into_async_nats(self) -> async_nats::jetstream::stream::RetentionPolicy {
        match self {
            Self::Limits => async_nats::jetstream::stream::RetentionPolicy::Limits,
            Self::Interest => async_nats::jetstream::stream::RetentionPolicy::Interest,
            Self::WorkQueue => async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
        }
    }
}

impl ConsumerDeliverPolicyKind {
    pub fn from_display(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "last" => Self::Last,
            "new" => Self::New,
            "lastpersubject" | "last_per_subject" | "last per subject" => Self::LastPerSubject,
            _ => Self::All,
        }
    }

    #[cfg(feature = "native")]
    fn from_async_nats(policy: async_nats::jetstream::consumer::DeliverPolicy) -> Self {
        match policy {
            async_nats::jetstream::consumer::DeliverPolicy::All => Self::All,
            async_nats::jetstream::consumer::DeliverPolicy::Last => Self::Last,
            async_nats::jetstream::consumer::DeliverPolicy::New => Self::New,
            async_nats::jetstream::consumer::DeliverPolicy::ByStartSequence { start_sequence } => {
                Self::ByStartSequence { start_sequence }
            }
            async_nats::jetstream::consumer::DeliverPolicy::ByStartTime { start_time } => {
                let start_time = start_time
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| start_time.to_string());
                Self::ByStartTime { start_time }
            }
            async_nats::jetstream::consumer::DeliverPolicy::LastPerSubject => Self::LastPerSubject,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::All => "All".to_string(),
            Self::Last => "Last".to_string(),
            Self::New => "New".to_string(),
            Self::ByStartSequence { start_sequence } => {
                format!("By start sequence ({start_sequence})")
            }
            Self::ByStartTime { start_time } => format!("By start time ({start_time})"),
            Self::LastPerSubject => "Last per subject".to_string(),
        }
    }

    #[cfg(feature = "native")]
    fn into_async_nats(self) -> Result<async_nats::jetstream::consumer::DeliverPolicy, String> {
        match self {
            Self::All => Ok(async_nats::jetstream::consumer::DeliverPolicy::All),
            Self::Last => Ok(async_nats::jetstream::consumer::DeliverPolicy::Last),
            Self::New => Ok(async_nats::jetstream::consumer::DeliverPolicy::New),
            Self::ByStartSequence { start_sequence } => Ok(
                async_nats::jetstream::consumer::DeliverPolicy::ByStartSequence { start_sequence },
            ),
            Self::ByStartTime { start_time } => {
                let start_time = time::OffsetDateTime::parse(
                    &start_time,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|e| format!("Invalid deliver policy start time (use RFC3339): {e}"))?;
                Ok(async_nats::jetstream::consumer::DeliverPolicy::ByStartTime { start_time })
            }
            Self::LastPerSubject => {
                Ok(async_nats::jetstream::consumer::DeliverPolicy::LastPerSubject)
            }
        }
    }
}

impl ConsumerAckPolicyKind {
    pub fn from_display(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "all" => Self::All,
            "none" => Self::None,
            _ => Self::Explicit,
        }
    }

    #[cfg(feature = "native")]
    fn into_async_nats(self) -> async_nats::jetstream::consumer::AckPolicy {
        match self {
            Self::Explicit => async_nats::jetstream::consumer::AckPolicy::Explicit,
            Self::All => async_nats::jetstream::consumer::AckPolicy::All,
            Self::None => async_nats::jetstream::consumer::AckPolicy::None,
        }
    }
}

impl StreamConfigInput {
    #[cfg(feature = "native")]
    pub fn into_async_nats(self) -> async_nats::jetstream::stream::Config {
        let mut config = async_nats::jetstream::stream::Config {
            name: self.name,
            subjects: self.subjects,
            storage: self.storage.into_async_nats(),
            retention: self.retention.into_async_nats(),
            ..Default::default()
        };
        if let Some(max_messages) = self.max_messages {
            config.max_messages = max_messages;
        }
        if let Some(max_bytes) = self.max_bytes {
            config.max_bytes = max_bytes;
        }
        if let Some(max_age) = self.max_age {
            config.max_age = max_age;
        }
        if let Some(num_replicas) = self.num_replicas {
            config.num_replicas = num_replicas;
        }
        config.description = self.description;
        config
    }
}

impl ConsumerConfigInput {
    #[cfg(feature = "native")]
    pub fn into_async_nats_pull(
        self,
    ) -> Result<async_nats::jetstream::consumer::pull::Config, String> {
        let deliver_policy = self.deliver_policy.into_async_nats()?;
        let mut config = async_nats::jetstream::consumer::pull::Config {
            name: (!self.name.trim().is_empty()).then_some(self.name),
            durable_name: self.durable_name,
            filter_subject: self.filter_subject.unwrap_or_default(),
            deliver_policy,
            ack_policy: self.ack_policy.into_async_nats(),
            description: self.description,
            ..Default::default()
        };
        if let Some(max_deliver) = self.max_deliver {
            config.max_deliver = max_deliver;
        }
        if let Some(max_ack_pending) = self.max_ack_pending {
            config.max_ack_pending = max_ack_pending;
        }
        Ok(config)
    }
}

impl KvBucketConfigInput {
    #[cfg(feature = "native")]
    pub fn into_async_nats(self) -> async_nats::jetstream::kv::Config {
        let mut config = async_nats::jetstream::kv::Config {
            bucket: self.bucket,
            history: self.history,
            storage: self.storage.into_async_nats(),
            description: self.description.unwrap_or_default(),
            ..Default::default()
        };
        if let Some(max_value_size) = self.max_value_size {
            config.max_value_size = max_value_size;
        }
        if let Some(max_bytes) = self.max_bytes {
            config.max_bytes = max_bytes;
        }
        if let Some(max_age) = self.max_age {
            config.max_age = max_age;
        }
        if let Some(num_replicas) = self.num_replicas {
            config.num_replicas = num_replicas;
        }
        config
    }
}

impl ObjectStoreBucketConfigInput {
    #[cfg(feature = "native")]
    pub fn into_async_nats(self) -> async_nats::jetstream::object_store::Config {
        let mut config = async_nats::jetstream::object_store::Config {
            bucket: self.bucket,
            storage: self.storage.into_async_nats(),
            description: self.description,
            ..Default::default()
        };
        if let Some(max_bytes) = self.max_bytes {
            config.max_bytes = max_bytes;
        }
        if let Some(num_replicas) = self.num_replicas {
            config.num_replicas = num_replicas;
        }
        config
    }
}

impl KvBucketInfo {
    #[cfg(feature = "native")]
    pub fn from_status(status: &async_nats::jetstream::kv::bucket::Status) -> Self {
        Self {
            bucket: status.bucket().to_string(),
            stored_history_values: status.values(),
            history_depth: status.history(),
            max_age_secs: status.max_age().as_secs(),
            max_age_nanos: status.max_age().as_nanos() as u64,
            description: status.info.config.description.clone().unwrap_or_default(),
            storage: format!("{:?}", status.info.config.storage),
            bytes: status.info.state.bytes,
            max_bytes: status.info.config.max_bytes,
            max_value_size: i64::from(status.info.config.max_message_size),
            num_replicas: status.info.config.num_replicas,
        }
    }
}

#[cfg(feature = "native")]
fn format_rfc3339(value: time::OffsetDateTime) -> String {
    value
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| value.to_string())
}

impl KvEntryInfo {
    #[cfg(feature = "native")]
    pub fn from_entry(entry: &async_nats::jetstream::kv::Entry) -> Self {
        Self {
            bucket: entry.bucket.clone(),
            key: entry.key.clone(),
            value: entry.value.to_vec(),
            revision: Some(entry.revision),
            delta: Some(entry.delta),
            created: Some(format_rfc3339(entry.created)),
            operation: Some(format!("{:?}", entry.operation)),
        }
    }

    pub fn missing(bucket: String, key: String) -> Self {
        Self {
            bucket,
            key,
            value: Vec::new(),
            revision: None,
            delta: None,
            created: None,
            operation: None,
        }
    }
}

impl KvHistoryItem {
    #[cfg(feature = "native")]
    pub fn from_entry(entry: &async_nats::jetstream::kv::Entry) -> Self {
        Self {
            key: entry.key.clone(),
            value: entry.value.to_vec(),
            revision: entry.revision,
            delta: entry.delta,
            created: format_rfc3339(entry.created),
            operation: format!("{:?}", entry.operation),
        }
    }
}

impl ObjectStoreBucketInfo {
    #[cfg(feature = "native")]
    pub fn from_stream_info(bucket: String, info: &async_nats::jetstream::stream::Info) -> Self {
        Self {
            bucket,
            description: info.config.description.clone().unwrap_or_default(),
            storage: format!("{:?}", info.config.storage),
            bytes: info.state.bytes,
            max_bytes: info.config.max_bytes,
            object_count: info.state.messages,
            num_replicas: info.config.num_replicas,
        }
    }
}

impl ObjectStoreObjectInfo {
    #[cfg(feature = "native")]
    pub fn from_info(info: &async_nats::jetstream::object_store::ObjectInfo) -> Self {
        Self {
            bucket: info.bucket.clone(),
            name: info.name.clone(),
            description: info.description.clone().unwrap_or_default(),
            size: info.size,
            chunks: info.chunks,
            modified: info.modified.map(format_rfc3339),
            digest: info.digest.clone(),
        }
    }
}

impl ServerInfoSnapshot {
    #[cfg(feature = "native")]
    pub fn from_info(info: &async_nats::ServerInfo) -> Self {
        Self {
            server_id: info.server_id.clone(),
            server_name: info.server_name.clone(),
            version: info.version.clone(),
            host: info.host.clone(),
            port: info.port,
            proto: info.proto,
            go: info.go.clone(),
            max_payload: info.max_payload,
            client_id: info.client_id,
            auth_required: info.auth_required,
            tls_required: info.tls_required,
            connect_urls: info.connect_urls.clone(),
        }
    }
}

impl JetStreamAccountLimitsSnapshot {
    #[cfg(feature = "native")]
    pub fn from_limits(limits: async_nats::jetstream::account::Limits) -> Self {
        Self {
            max_memory: limits.max_memory,
            max_storage: limits.max_storage,
            max_streams: limits.max_streams,
            max_consumers: limits.max_consumers,
            max_ack_pending: limits.max_ack_pending,
            memory_max_stream_bytes: limits.memory_max_stream_bytes,
            storage_max_stream_bytes: limits.storage_max_stream_bytes,
            max_bytes_required: limits.max_bytes_required,
        }
    }
}

impl JetStreamAccountInfoSnapshot {
    #[cfg(feature = "native")]
    pub fn from_account(account: async_nats::jetstream::account::Account) -> Self {
        Self {
            memory: account.memory,
            storage: account.storage,
            streams: account.streams,
            consumers: account.consumers,
            domain: account.domain,
            limits: JetStreamAccountLimitsSnapshot::from_limits(account.limits),
            api_total: account.requests.total,
            api_errors: account.requests.errors,
        }
    }
}

impl StreamInfo {
    #[cfg(feature = "native")]
    pub fn from_info(info: &async_nats::jetstream::stream::Info) -> Self {
        Self {
            name: info.config.name.clone(),
            subjects: info.config.subjects.clone(),
            storage: format!("{:?}", info.config.storage),
            retention: format!("{:?}", info.config.retention),
            messages: info.state.messages,
            bytes: info.state.bytes,
            first_sequence: info.state.first_sequence,
            last_sequence: info.state.last_sequence,
            consumer_count: info.state.consumer_count,
        }
    }
}

impl StreamMessageInfo {
    #[cfg(feature = "native")]
    pub fn from_stream_message(msg: &async_nats::jetstream::message::StreamMessage) -> Self {
        let mut headers = Vec::new();
        for (name, values) in msg.headers.iter() {
            for value in values.iter() {
                headers.push((name.to_string(), value.to_string()));
            }
        }
        Self {
            sequence: msg.sequence,
            subject: msg.subject.to_string(),
            payload: msg.payload.to_vec(),
            headers,
            time: msg
                .time
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

impl ConsumerInfo {
    #[cfg(feature = "native")]
    pub fn from_info(info: &async_nats::jetstream::consumer::Info) -> Self {
        Self {
            name: info.name.clone(),
            stream_name: info.stream_name.clone(),
            durable_name: info.config.durable_name.clone(),
            filter_subject: (!info.config.filter_subject.is_empty())
                .then(|| info.config.filter_subject.clone()),
            deliver_policy: ConsumerDeliverPolicyKind::from_async_nats(info.config.deliver_policy),
            ack_policy: serde_json::to_value(info.config.ack_policy)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_else(|| format!("{:?}", info.config.ack_policy)),
            max_deliver: info.config.max_deliver,
            max_ack_pending: info.config.max_ack_pending,
            description: info.config.description.clone(),
            deliver_subject: info.config.deliver_subject.clone(),
            num_pending: info.num_pending,
            num_ack_pending: info.num_ack_pending as u64,
            num_waiting: info.num_waiting as u64,
            num_redelivered: info.num_redelivered as u64,
            push_bound: info.push_bound,
        }
    }
}

#[cfg(all(test, feature = "native"))]
mod tests;
