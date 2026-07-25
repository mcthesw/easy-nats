mod fixtures;
mod kv;
mod monitoring;
mod object_store;
mod pubsub;
mod streams;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::time::Duration;

use web_time::Instant;

use crate::command::BackendCommand;
use crate::event::{BackendEvent, BackendOperation};
use crate::models::{
    ConsumerInfo, KvBucketInfo, KvHistoryItem, ObjectStoreBucketInfo, ObjectStoreObjectInfo,
    StreamInfo, StreamMessageInfo,
};

const EVENT_LIMIT: usize = 512;
pub(super) const STREAM_MESSAGE_LIMIT: usize = 100;
const SYNTHETIC_INTERVAL: Duration = Duration::from_millis(1_300);

pub struct BackendHandle {
    state: RefCell<DemoState>,
}

impl BackendHandle {
    pub fn spawn() -> Self {
        Self {
            state: RefCell::new(DemoState::new()),
        }
    }

    pub fn send(&self, command: BackendCommand) {
        self.state.borrow_mut().handle(command);
    }

    pub fn try_recv(&mut self) -> Option<BackendEvent> {
        let state = self.state.get_mut();
        state.enqueue_synthetic_message();
        state.events.pop_front()
    }

    pub fn drain_events(&mut self) -> Vec<BackendEvent> {
        let state = self.state.get_mut();
        state.enqueue_synthetic_message();
        state.events.drain(..).collect()
    }

    /// Return the delay until the next scheduled synthetic demo message.
    pub fn next_wakeup(&self) -> Option<Duration> {
        Some(
            self.state
                .borrow()
                .next_synthetic
                .saturating_duration_since(Instant::now()),
        )
    }
}

pub(super) struct DemoState {
    events: VecDeque<BackendEvent>,
    subscriptions: HashMap<(u64, u64), HashSet<String>>,
    streams: BTreeMap<String, StreamInfo>,
    stream_messages: HashMap<String, Vec<StreamMessageInfo>>,
    consumers: HashMap<String, BTreeMap<String, ConsumerInfo>>,
    kv_buckets: BTreeMap<String, KvBucketInfo>,
    kv_entries: HashMap<String, BTreeMap<String, Vec<KvHistoryItem>>>,
    object_buckets: BTreeMap<String, ObjectStoreBucketInfo>,
    objects: HashMap<String, BTreeMap<String, ObjectStoreObjectInfo>>,
    revision: u64,
    synthetic_count: u64,
    next_synthetic: Instant,
}

impl DemoState {
    fn new() -> Self {
        let stream = fixtures::stream();
        let consumer = fixtures::consumer();
        let kv_bucket = fixtures::kv_bucket();
        let object_bucket = fixtures::object_bucket();
        Self {
            events: VecDeque::new(),
            subscriptions: HashMap::new(),
            streams: [(stream.name.clone(), stream)].into(),
            stream_messages: [("DEMO_EVENTS".into(), fixtures::stream_messages())].into(),
            consumers: [(
                "DEMO_EVENTS".into(),
                [(consumer.name.clone(), consumer)].into(),
            )]
            .into(),
            kv_buckets: [(kv_bucket.bucket.clone(), kv_bucket)].into(),
            kv_entries: [(
                "demo_config".into(),
                fixtures::kv_entries().into_iter().collect(),
            )]
            .into(),
            object_buckets: [(object_bucket.bucket.clone(), object_bucket)].into(),
            objects: [(
                "demo_assets".into(),
                fixtures::objects()
                    .into_iter()
                    .map(|object| (object.name.clone(), object))
                    .collect(),
            )]
            .into(),
            revision: 3,
            synthetic_count: 0,
            next_synthetic: Instant::now() + Duration::from_millis(250),
        }
    }

    pub(super) fn push(&mut self, event: BackendEvent) {
        if self.events.len() == EVENT_LIMIT {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub(super) fn succeeded(&mut self, connection_id: u64, operation: BackendOperation) {
        self.push(BackendEvent::OperationSucceeded {
            connection_id,
            operation,
        });
    }

    fn handle(&mut self, command: BackendCommand) {
        match command {
            BackendCommand::Connect { config } => self.connect(config.id),
            BackendCommand::Disconnect { id } => self.disconnect(id),
            BackendCommand::Publish {
                connection_id,
                subject,
                payload,
                headers,
            } => self.publish(connection_id, subject, payload, headers),
            BackendCommand::Subscribe {
                connection_id,
                backend_id,
                subject,
                cancel: _,
            } => self.subscribe(connection_id, backend_id, subject),
            BackendCommand::Unsubscribe {
                connection_id,
                backend_id,
                subject,
            } => self.unsubscribe(connection_id, backend_id, subject),
            BackendCommand::Request {
                connection_id,
                backend_id,
                request_id,
                subject,
                payload,
                headers: _,
                timeout_ms: _,
            } => self.request(connection_id, backend_id, request_id, subject, payload),
            BackendCommand::Reply {
                connection_id,
                backend_id,
                reply_id,
                reply_to,
                ..
            } => self.reply(connection_id, backend_id, reply_id, reply_to),
            BackendCommand::ListStreams { connection_id } => self.list_streams(connection_id),
            BackendCommand::CreateStream {
                connection_id,
                config,
            } => self.upsert_stream(connection_id, config),
            BackendCommand::UpdateStream {
                connection_id,
                config,
            } => self.upsert_stream(connection_id, config),
            BackendCommand::DeleteStream {
                connection_id,
                name,
            } => self.delete_stream(connection_id, name),
            BackendCommand::PurgeStream {
                connection_id,
                name,
                filter,
            } => self.purge_stream(connection_id, name, filter),
            BackendCommand::GetStreamMessages {
                connection_id,
                stream,
                start_sequence,
                subject_filter,
                start_time,
                batch_size,
            } => self.get_stream_messages(
                connection_id,
                stream,
                start_sequence,
                subject_filter,
                start_time,
                batch_size,
            ),
            BackendCommand::DeleteStreamMessage {
                connection_id,
                stream,
                sequence,
            } => self.delete_stream_message(connection_id, stream, sequence),
            BackendCommand::ListConsumers {
                connection_id,
                stream,
            } => self.list_consumers(connection_id, stream),
            BackendCommand::CreateConsumer {
                connection_id,
                stream,
                config,
            } => self.upsert_consumer(connection_id, stream, config),
            BackendCommand::UpdateConsumer {
                connection_id,
                stream,
                config,
            } => self.upsert_consumer(connection_id, stream, config),
            BackendCommand::DeleteConsumer {
                connection_id,
                stream,
                name,
            } => self.delete_consumer(connection_id, stream, name),
            BackendCommand::FetchConsumerMessages {
                connection_id,
                stream,
                consumer,
                batch,
            } => self.fetch_consumer_messages(connection_id, stream, consumer, batch),
            BackendCommand::ListKvBuckets { connection_id } => self.list_kv_buckets(connection_id),
            BackendCommand::CreateKvBucket {
                connection_id,
                config,
            } => self.upsert_kv_bucket(connection_id, config),
            BackendCommand::UpdateKvBucket {
                connection_id,
                config,
            } => self.upsert_kv_bucket(connection_id, config),
            BackendCommand::DeleteKvBucket {
                connection_id,
                bucket,
            } => self.delete_kv_bucket(connection_id, bucket),
            BackendCommand::ListKvKeys {
                connection_id,
                bucket,
                generation,
                ..
            } => self.list_kv_keys(connection_id, bucket, generation),
            BackendCommand::GetKvEntry {
                connection_id,
                bucket,
                key,
            } => self.get_kv_entry(connection_id, bucket, key),
            BackendCommand::PutKvEntry {
                connection_id,
                bucket,
                key,
                value,
            } => self.put_kv_entry(connection_id, bucket, key, value),
            BackendCommand::DeleteKvEntry {
                connection_id,
                bucket,
                key,
            } => self.delete_kv_entry(connection_id, bucket, key),
            BackendCommand::PurgeKvEntry {
                connection_id,
                bucket,
                key,
            } => self.purge_kv_entry(connection_id, bucket, key),
            BackendCommand::GetKvHistory {
                connection_id,
                bucket,
                key,
            } => self.get_kv_history(connection_id, bucket, key),
            BackendCommand::ListObjectStoreBuckets { connection_id } => {
                self.list_object_store_buckets(connection_id);
            }
            BackendCommand::CreateObjectStoreBucket {
                connection_id,
                config,
            } => self.create_object_store_bucket(connection_id, config),
            BackendCommand::DeleteObjectStoreBucket {
                connection_id,
                bucket,
            } => self.delete_object_store_bucket(connection_id, bucket),
            BackendCommand::ListObjects {
                connection_id,
                bucket,
            } => self.list_objects(connection_id, bucket),
            BackendCommand::UploadObject {
                connection_id,
                bucket,
                name,
                data,
            } => self.upload_object(connection_id, bucket, name, data),
            BackendCommand::DownloadObject {
                connection_id,
                bucket,
                name,
                file_path,
            } => self.download_object(connection_id, bucket, name, file_path),
            BackendCommand::DeleteObject {
                connection_id,
                bucket,
                name,
            } => self.delete_object(connection_id, bucket, name),
            BackendCommand::GetServerInfo { connection_id } => self.get_server_info(connection_id),
            BackendCommand::GetJetStreamAccountInfo { connection_id } => {
                self.get_jetstream_account_info(connection_id);
            }
            BackendCommand::FetchMetrics {
                connection_id,
                endpoint,
            } => self.fetch_metrics(connection_id, endpoint),
            BackendCommand::FetchClientStatusPage {
                connection_id,
                endpoint,
                query,
            } => self.fetch_client_status_page(connection_id, endpoint, query),
            BackendCommand::FetchClientStatusDetail {
                connection_id,
                endpoint,
                query,
            } => self.fetch_client_status_detail(connection_id, endpoint, query),
        }
    }
}

pub(super) fn subject_matches(pattern: &str, subject: &str) -> bool {
    let pattern_tokens: Vec<_> = pattern.split('.').collect();
    let subject_tokens: Vec<_> = subject.split('.').collect();
    let mut index = 0;
    while index < pattern_tokens.len() {
        match pattern_tokens[index] {
            ">" => {
                return index + 1 == pattern_tokens.len() && index < subject_tokens.len();
            }
            "*" if index < subject_tokens.len() => {}
            token if subject_tokens.get(index) == Some(&token) => {}
            _ => return false,
        }
        index += 1;
    }
    index == subject_tokens.len()
}

#[cfg(test)]
mod tests;
