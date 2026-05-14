mod client_status_state;
mod metrics_state;
mod tab_kind;

pub use client_status_state::ClientStatusState;
pub use metrics_state::MetricsState;
pub use tab_kind::{AppTabViewer, TabKind};

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Instant, SystemTime};

use eframe::egui;
use nats_backend::{
    ConsumerInfo, JetStreamAccountInfoSnapshot, KvBucketInfo, KvHistoryItem, ObjectStoreBucketInfo,
    ObjectStoreObjectInfo, ServerInfoSnapshot, StreamInfo, StreamMessageInfo,
};

use crate::format::PayloadFormat;
use crate::proto::ProtoViewState;
use crate::schema::{SchemaSelector, SchemaSourceKind, ValidationPolicy};
use crate::theme::ThemeId;

/// Auto-refresh configuration for periodic data reloading.
#[derive(Debug)]
pub struct AutoRefresh {
    pub enabled: bool,
    pub interval_secs: u64,
    pub last_refresh: Instant,
}

impl Default for AutoRefresh {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: 5,
            last_refresh: Instant::now(),
        }
    }
}

impl AutoRefresh {
    pub const INTERVALS: &[u64] = &[1, 5, 10, 30];

    /// Returns true if the interval has elapsed and refresh is needed.
    pub fn should_refresh(&self) -> bool {
        self.enabled && self.last_refresh.elapsed().as_secs() >= self.interval_secs
    }

    /// Mark that a refresh just happened.
    pub fn mark_refreshed(&mut self) {
        self.last_refresh = Instant::now();
    }
}

#[derive(Debug)]
pub enum TabAction {
    OpenStreamPublish {
        connection_id: u64,
        stream_name: String,
        subject: String,
    },
    OpenConsumerCreate {
        connection_id: u64,
        stream_name: String,
    },
    OpenConsumerEdit {
        connection_id: u64,
        stream_name: String,
        consumer_info: ConsumerInfo,
    },
    OpenKvBucketEdit {
        connection_id: u64,
        bucket_info: KvBucketInfo,
    },
    OpenKvEntryCreate {
        connection_id: u64,
        bucket_name: String,
        initial_key: String,
    },
    ConfirmDeleteKvBucket {
        connection_id: u64,
        bucket_name: String,
    },
    ConfirmDeleteObjStoreBucket {
        connection_id: u64,
        bucket_name: String,
    },
    CloseOtherTabs {
        keep_tab_id: egui::Id,
    },
    CloseAllTabs,
    CloseTabsToRight {
        of_tab_id: egui::Id,
    },
    OpenConnectionEditor,
    ApplyTheme {
        theme_id: ThemeId,
    },
    OpenMessageSchemas,
    AddMessageSchemaSource {
        name: String,
        kind: SchemaSourceKind,
        path: String,
    },
    RemoveMessageSchemaSource {
        source_id: u64,
    },
    ReloadMessageSchemaSource {
        source_id: u64,
    },
    SetMessageSchemaSourceEnabled {
        source_id: u64,
        enabled: bool,
    },
    AddMessageSchemaBinding {
        name: String,
        connection_id: Option<u64>,
        subject_pattern: String,
        source_id: u64,
        selector: SchemaSelector,
        policy: ValidationPolicy,
    },
    RemoveMessageSchemaBinding {
        binding_id: u64,
    },
    SetMessageSchemaBindingEnabled {
        binding_id: u64,
        enabled: bool,
    },
    ScanSearchWorkspaceKvValues {
        source_id: SearchSourceId,
    },
    NavigateSearchResult {
        locator: SearchResultLocator,
    },
    RecordTopic {
        topic: String,
    },
}

#[derive(Debug)]
pub struct PublisherState {
    pub subject: String,
    pub subject_suggestion_idx: Option<usize>,
    pub payload: String,
    pub headers: Vec<(String, String)>,
    pub timeout_ms: String,
    pub response: Option<ResponseData>,
    pub waiting: bool,
    pub response_format: PayloadFormat,
    pub proto_view: ProtoViewState,
}

impl Default for PublisherState {
    fn default() -> Self {
        Self {
            subject: String::new(),
            subject_suggestion_idx: None,
            payload: String::new(),
            headers: Vec::new(),
            timeout_ms: "5000".to_string(),
            response: None,
            waiting: false,
            response_format: PayloadFormat::Auto,
            proto_view: ProtoViewState::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub subject: Option<String>,
    pub payload: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    pub id: u64,
    pub subject: String,
    pub reply: Option<String>,
    pub headers: Vec<(String, String)>,
    pub payload: Vec<u8>,
    pub timestamp: SystemTime,
}

pub type SubscriberListRow = (usize, String, String);
pub type CachedSubscriberRows = (u64, Option<String>, SearchCacheKey, Vec<SubscriberListRow>);
pub type StreamListRow = (usize, String);
pub type CachedStreamRows = (u64, SearchCacheKey, Vec<StreamListRow>);

#[derive(Debug, Clone)]
pub struct CachedKvKeyRows {
    pub generation: u64,
    pub cache_key: SearchCacheKey,
    pub selected_key: Option<String>,
    pub rows: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchCacheKey {
    pub query: String,
    pub primary: bool,
    pub secondary: bool,
}

impl SearchCacheKey {
    pub fn from_state(search: &ScopedSearchState) -> Self {
        Self {
            query: search.normalized_query(),
            primary: search.primary,
            secondary: search.secondary,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScopedSearchState {
    pub query: String,
    pub primary: bool,
    pub secondary: bool,
}

impl ScopedSearchState {
    pub fn new(primary: bool, secondary: bool) -> Self {
        Self {
            query: String::new(),
            primary,
            secondary,
        }
    }

    pub fn is_active(&self) -> bool {
        !self.query.trim().is_empty() && (self.primary || self.secondary)
    }

    pub fn normalized_query(&self) -> String {
        self.query.trim().to_lowercase()
    }
}

impl Default for ScopedSearchState {
    fn default() -> Self {
        Self::new(true, true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SearchSourceId {
    Kv {
        connection_id: u64,
        bucket_name: String,
    },
    Stream {
        connection_id: u64,
        stream_name: String,
    },
    Subscriber {
        connection_id: u64,
        backend_id: u64,
    },
}

impl SearchSourceId {
    pub fn fallback_label(&self) -> String {
        match self {
            Self::Kv {
                connection_id,
                bucket_name,
            } => format!("KV {bucket_name} #{connection_id}"),
            Self::Stream {
                connection_id,
                stream_name,
            } => format!("Stream {stream_name} #{connection_id}"),
            Self::Subscriber {
                connection_id,
                backend_id,
            } => format!("Subscriber #{backend_id} ({connection_id})"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SearchSourceCoverage {
    Kv {
        loaded_keys: usize,
        fetched_values: usize,
        scanning: usize,
        can_scan_more: bool,
    },
    Stream {
        messages: usize,
    },
    Subscriber {
        messages: usize,
        max_messages: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchField {
    Key,
    Value,
    Subject,
    Payload,
}

#[derive(Debug, Clone)]
pub enum SearchResultLocator {
    KvKey {
        connection_id: u64,
        bucket_name: String,
        key: String,
    },
    StreamMessage {
        connection_id: u64,
        stream_name: String,
        sequence: u64,
    },
    SubscriberMessage {
        connection_id: u64,
        backend_id: u64,
        message_id: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchSourceKind {
    Kv,
    Stream,
    Subscriber,
}

#[derive(Debug, Clone)]
pub struct SearchSourceSummary {
    pub id: SearchSourceId,
    pub label: String,
    pub generation: u64,
    pub coverage: SearchSourceCoverage,
    pub kind: SearchSourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultIdentity {
    pub source_id: SearchSourceId,
    pub generation: u64,
    pub field: SearchField,
    pub item_id: String,
}

#[derive(Debug, Clone)]
pub struct SearchWorkspaceResult {
    pub identity: SearchResultIdentity,
    pub source_label: String,
    pub field: SearchField,
    pub item_label: String,
    pub snippet: String,
    pub preview: String,
    pub locator: SearchResultLocator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchWorkspaceCacheKey {
    pub query: String,
    pub primary: bool,
    pub secondary: bool,
    pub sources: Vec<(SearchSourceId, Option<u64>)>,
}

pub type CachedSearchWorkspaceResults = (SearchWorkspaceCacheKey, Vec<SearchWorkspaceResult>);

#[derive(Debug)]
pub struct SearchWorkspaceState {
    pub query: String,
    pub primary: bool,
    pub secondary: bool,
    pub selected_sources: Vec<SearchSourceId>,
    pub selected_result: Option<SearchResultIdentity>,
    pub selected_preview: Option<SearchWorkspaceResult>,
    pub cached_results: Option<CachedSearchWorkspaceResults>,
}

#[derive(Debug)]
pub struct MessageSchemasState {
    pub source_name: String,
    pub source_kind: SchemaSourceKind,
    pub source_path: String,
    pub binding_name: String,
    pub binding_connection_id: Option<u64>,
    pub binding_subject_pattern: String,
    pub binding_source_id: Option<u64>,
    pub binding_schema_entry: String,
    pub binding_policy: ValidationPolicy,
    pub last_error: Option<String>,
}

impl Default for MessageSchemasState {
    fn default() -> Self {
        Self {
            source_name: String::new(),
            source_kind: SchemaSourceKind::Protobuf,
            source_path: String::new(),
            binding_name: String::new(),
            binding_connection_id: None,
            binding_subject_pattern: String::new(),
            binding_source_id: None,
            binding_schema_entry: String::new(),
            binding_policy: ValidationPolicy::Inspect,
            last_error: None,
        }
    }
}

impl Default for SearchWorkspaceState {
    fn default() -> Self {
        Self {
            query: String::new(),
            primary: true,
            secondary: true,
            selected_sources: Vec::new(),
            selected_result: None,
            selected_preview: None,
            cached_results: None,
        }
    }
}

#[derive(Debug)]
pub struct SubjectSubscription {
    pub subject: String,
    pub active: bool,
}

#[derive(Debug)]
pub struct SubscriberState {
    pub subject_input: String,
    pub subject_suggestion_idx: Option<usize>,
    pub subscriptions: Vec<SubjectSubscription>,
    pub messages: VecDeque<ReceivedMessage>,
    pub next_message_id: u64,
    pub max_messages: usize,
    pub selected_idx: Option<usize>,
    pub payload_format: PayloadFormat,
    /// When set, only display messages matching this subject.
    pub subject_filter: Option<String>,
    pub cache_generation: u64,
    pub cached_filtered: Option<CachedSubscriberRows>,
    pub search: ScopedSearchState,
    pub proto_view: ProtoViewState,
}

impl Default for SubscriberState {
    fn default() -> Self {
        Self {
            subject_input: String::new(),
            subject_suggestion_idx: None,
            subscriptions: Vec::new(),
            messages: VecDeque::new(),
            next_message_id: 1,
            max_messages: 1000,
            selected_idx: None,
            payload_format: PayloadFormat::Auto,
            subject_filter: None,
            cache_generation: 0,
            cached_filtered: None,
            search: ScopedSearchState::default(),
            proto_view: ProtoViewState::default(),
        }
    }
}

impl SubscriberState {
    pub fn push_messages<I>(&mut self, messages: I)
    where
        I: IntoIterator<Item = ReceivedMessage>,
    {
        let mut pushed = false;
        for msg in messages {
            self.push_message_without_invalidation(msg);
            pushed = true;
        }
        if pushed {
            self.invalidate_filtered_cache();
        }
    }

    fn push_message_without_invalidation(&mut self, mut msg: ReceivedMessage) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
            if let Some(idx) = self.selected_idx {
                self.selected_idx = idx.checked_sub(1);
            }
        }
        msg.id = self.next_message_id;
        self.next_message_id = self.next_message_id.wrapping_add(1).max(1);
        self.messages.push_back(msg);
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.selected_idx = None;
        self.invalidate_filtered_cache();
    }

    pub fn invalidate_filtered_cache(&mut self) {
        self.cache_generation = self.cache_generation.wrapping_add(1);
        self.cached_filtered = None;
    }
}

#[derive(Debug)]
pub struct StreamState {
    pub info: Option<StreamInfo>,
    pub messages: Vec<StreamMessageInfo>,
    pub selected_msg: Option<usize>,
    pub payload_format: PayloadFormat,
    pub start_seq: String,
    pub subject_filter: String,
    pub start_time: String,
    pub batch_size: String,
    pub fetching: bool,
    pub purge_subject: String,
    pub consumers: Vec<ConsumerInfo>,
    pub consumers_fetching: bool,
    pub auto_refresh: AutoRefresh,
    pub proto_view: ProtoViewState,
    pub consumer_fetched: std::collections::HashMap<String, Vec<StreamMessageInfo>>,
    pub consumer_fetching: std::collections::HashSet<String>,
    pub search: ScopedSearchState,
    pub search_generation: u64,
    pub cached_filtered: Option<CachedStreamRows>,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            info: None,
            messages: Vec::new(),
            selected_msg: None,
            payload_format: PayloadFormat::Auto,
            start_seq: String::new(),
            subject_filter: String::new(),
            start_time: String::new(),
            batch_size: "50".to_string(),
            fetching: false,
            purge_subject: String::new(),
            consumers: Vec::new(),
            consumers_fetching: false,
            auto_refresh: AutoRefresh::default(),
            proto_view: ProtoViewState::default(),
            consumer_fetched: std::collections::HashMap::new(),
            consumer_fetching: std::collections::HashSet::new(),
            search: ScopedSearchState::default(),
            search_generation: 0,
            cached_filtered: None,
        }
    }
}

#[derive(Debug)]
pub struct KvBucketState {
    pub info: Option<KvBucketInfo>,
    pub keys: Vec<String>,
    pub selected_key: Option<String>,
    pub search: ScopedSearchState,
    pub keys_complete: bool,
    pub search_more_requested: bool,
    pub value_search_scanning: usize,
    pub value_search_pending: HashSet<String>,
    pub value_search_cursor: usize,
    pub loading_entries: bool,
    pub loading_entry: bool,
    pub loading_history: bool,
    pub load_generation: u64,
    pub search_generation: u64,
    pub cached_filtered_keys: Option<CachedKvKeyRows>,
    pub entry_key: String,
    pub entry_value: String,
    pub fetched_values: HashMap<String, String>,
    pub entry_revision: Option<u64>,
    pub entry_operation: Option<String>,
    pub entry_created: Option<String>,
    pub editor_format: PayloadFormat,
    pub history: Vec<KvHistoryItem>,
    pub history_format: PayloadFormat,
    pub show_history: bool,
    pub auto_refresh: AutoRefresh,
    pub editor_proto_view: ProtoViewState,
    pub history_proto_view: ProtoViewState,
}

impl Default for KvBucketState {
    fn default() -> Self {
        Self {
            info: None,
            keys: Vec::new(),
            selected_key: None,
            search: ScopedSearchState::default(),
            keys_complete: false,
            search_more_requested: false,
            value_search_scanning: 0,
            value_search_pending: HashSet::new(),
            value_search_cursor: 0,
            loading_entries: false,
            loading_entry: false,
            loading_history: false,
            load_generation: 0,
            search_generation: 0,
            cached_filtered_keys: None,
            entry_key: String::new(),
            entry_value: String::new(),
            fetched_values: HashMap::new(),
            entry_revision: None,
            entry_operation: None,
            entry_created: None,
            editor_format: PayloadFormat::Auto,
            history: Vec::new(),
            history_format: PayloadFormat::Auto,
            show_history: false,
            auto_refresh: AutoRefresh::default(),
            editor_proto_view: ProtoViewState::default(),
            history_proto_view: ProtoViewState::default(),
        }
    }
}

impl KvBucketState {
    pub(crate) fn invalidate_filtered_key_cache(&mut self) {
        self.cached_filtered_keys = None;
    }
}

#[derive(Debug, Default)]
pub struct ObjectStoreBucketState {
    pub info: Option<ObjectStoreBucketInfo>,
    pub objects: Vec<ObjectStoreObjectInfo>,
    pub selected_object: Option<String>,
    pub object_filter: String,
    pub loading_objects: bool,
    pub delete_confirm: bool,
    pub auto_refresh: AutoRefresh,
}

#[derive(Debug, Default)]
pub struct ServerInfoState {
    pub server_info: Option<ServerInfoSnapshot>,
    pub account_info: Option<JetStreamAccountInfoSnapshot>,
    pub loading: bool,
}
