use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Instant, SystemTime};

use eframe::egui;
use egui_dock::TabViewer;
use egui_dock::tab_viewer::OnCloseResponse;
use nats_backend::MetricsSnapshot;

use crate::format::PayloadFormat;
use crate::i18n::t;
use crate::proto::{ProtoSchemaManager, ProtoViewState};
use crate::theme::ThemeId;

use super::guard::TabGuard;

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
        consumer_json: serde_json::Value,
    },
    OpenKvBucketEdit {
        connection_id: u64,
        bucket_json: serde_json::Value,
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
    LoadProtoSchemas {
        dir: String,
    },
    ClearProtoSchemas,
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

impl SearchField {
    pub fn is_primary(self) -> bool {
        matches!(self, Self::Key | Self::Subject)
    }
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

#[derive(Debug, Clone)]
pub struct SearchRecordSnapshot {
    pub field: SearchField,
    pub item_id: String,
    pub item_label: String,
    pub text: String,
    pub snippet: String,
    pub locator: SearchResultLocator,
}

#[derive(Debug, Clone)]
pub struct SearchSourceSnapshot {
    pub id: SearchSourceId,
    pub label: String,
    pub generation: u64,
    pub coverage: SearchSourceCoverage,
    pub records: Vec<SearchRecordSnapshot>,
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
    pub fn push_message(&mut self, mut msg: ReceivedMessage) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
            if let Some(idx) = self.selected_idx {
                self.selected_idx = if idx == 0 { None } else { Some(idx - 1) };
            }
        }
        msg.id = self.next_message_id;
        self.next_message_id = self.next_message_id.wrapping_add(1).max(1);
        self.messages.push_back(msg);
        self.invalidate_filtered_cache();
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
    pub info: Option<serde_json::Value>,
    pub messages: Vec<serde_json::Value>,
    pub selected_msg: Option<usize>,
    pub payload_format: PayloadFormat,
    pub start_seq: String,
    pub subject_filter: String,
    pub start_time: String,
    pub batch_size: String,
    pub fetching: bool,
    pub purge_subject: String,
    pub consumers: Vec<serde_json::Value>,
    pub consumers_fetching: bool,
    pub auto_refresh: AutoRefresh,
    pub proto_view: ProtoViewState,
    pub consumer_fetched: std::collections::HashMap<String, Vec<serde_json::Value>>,
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
    pub info: Option<serde_json::Value>,
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
    pub entry_key: String,
    pub entry_value: String,
    pub fetched_values: HashMap<String, String>,
    pub entry_revision: Option<u64>,
    pub entry_operation: Option<String>,
    pub entry_created: Option<String>,
    pub editor_format: PayloadFormat,
    pub history: Vec<serde_json::Value>,
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

#[derive(Debug, Default)]
pub struct ObjectStoreBucketState {
    pub info: Option<serde_json::Value>,
    pub objects: Vec<serde_json::Value>,
    pub selected_object: Option<String>,
    pub object_filter: String,
    pub loading_objects: bool,
    pub delete_confirm: bool,
    pub auto_refresh: AutoRefresh,
}

#[derive(Debug, Default)]
pub struct ServerInfoState {
    pub server_info: Option<serde_json::Value>,
    pub account_info: Option<serde_json::Value>,
    pub loading: bool,
}

#[derive(Debug, Default)]
pub struct MetricsState {
    endpoint: String,
    latest_attempt: Option<MetricsSnapshot>,
    history: VecDeque<MetricsSnapshot>,
    pub loading: bool,
    pub auto_refresh: AutoRefresh,
}

impl MetricsState {
    const MAX_SAMPLES: usize = 120;

    pub fn with_endpoint(endpoint: String) -> Self {
        let mut state = Self {
            endpoint,
            ..Default::default()
        };
        state.auto_refresh.enabled = true;
        state.auto_refresh.interval_secs = 5;
        state
    }

    pub fn endpoint(&self) -> &str {
        self.endpoint.as_str()
    }

    pub fn endpoint_configured(&self) -> bool {
        !self.endpoint.trim().is_empty()
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        if self.endpoint == endpoint {
            return;
        }
        self.endpoint = endpoint;
        self.latest_attempt = None;
        self.history.clear();
        self.loading = false;
        self.auto_refresh.last_refresh = Instant::now();
    }

    pub fn begin_refresh(&mut self) {
        self.loading = true;
        self.auto_refresh.mark_refreshed();
    }

    pub fn apply_snapshot(&mut self, snapshot: MetricsSnapshot) {
        self.endpoint = snapshot.endpoint.clone();
        self.loading = false;
        if snapshot.has_any_data() {
            if self.history.len() >= Self::MAX_SAMPLES {
                self.history.pop_front();
            }
            self.history.push_back(snapshot.clone());
        }
        self.latest_attempt = Some(snapshot);
    }

    pub fn has_never_loaded(&self) -> bool {
        self.latest_attempt.is_none() && self.history.is_empty()
    }

    pub fn latest_attempt(&self) -> Option<&MetricsSnapshot> {
        self.latest_attempt.as_ref()
    }

    pub fn latest_data(&self) -> Option<&MetricsSnapshot> {
        self.latest_attempt
            .as_ref()
            .filter(|snapshot| snapshot.has_any_data())
            .or_else(|| self.history.back())
    }

    pub fn history(&self) -> &VecDeque<MetricsSnapshot> {
        &self.history
    }

    pub fn is_stale(&self) -> bool {
        self.latest_attempt
            .as_ref()
            .is_some_and(|snapshot| !snapshot.has_any_data())
            && !self.history.is_empty()
    }
}

#[derive(Debug)]
#[allow(dead_code)] // guard fields exist for RAII Drop cleanup, not direct reads
pub enum TabKind {
    Welcome,
    Publisher {
        connection_id: u64,
        connection_name: String,
        guard: TabGuard,
        backend_id: u64,
        state: PublisherState,
    },
    Subscriber {
        connection_id: u64,
        connection_name: String,
        guard: TabGuard,
        backend_id: u64,
        state: SubscriberState,
    },
    Stream {
        connection_id: u64,
        connection_name: String,
        stream_name: String,
        guard: TabGuard,
        state: StreamState,
    },
    KvBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
        guard: TabGuard,
        state: KvBucketState,
    },
    ObjectStoreBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
        guard: TabGuard,
        state: ObjectStoreBucketState,
    },
    ServerInfo {
        connection_id: u64,
        connection_name: String,
        guard: TabGuard,
        state: ServerInfoState,
    },
    Metrics {
        connection_id: u64,
        connection_name: String,
        state: MetricsState,
    },
    SearchWorkspace {
        state: SearchWorkspaceState,
    },
    Settings,
    LogViewer,
}

impl TabKind {
    pub fn title(&self) -> String {
        match self {
            TabKind::Welcome => t("common.tab_welcome").to_string(),
            TabKind::Publisher {
                connection_name,
                guard,
                ..
            } => {
                if let Some(id) = guard.display_id() {
                    format!(
                        "{} #{} ({})",
                        t("common.tab_publisher"),
                        id,
                        connection_name
                    )
                } else {
                    format!("{} ({})", t("common.tab_publisher"), connection_name)
                }
            }
            TabKind::Subscriber {
                connection_name,
                guard,
                ..
            } => {
                if let Some(id) = guard.display_id() {
                    format!(
                        "{} #{} ({})",
                        t("common.tab_subscriber"),
                        id,
                        connection_name
                    )
                } else {
                    format!("{} ({})", t("common.tab_subscriber"), connection_name)
                }
            }
            TabKind::Stream {
                connection_name,
                stream_name,
                ..
            } => {
                format!("{stream_name} ({connection_name})")
            }
            TabKind::KvBucket {
                connection_name,
                bucket_name,
                ..
            } => {
                format!("{bucket_name} ({connection_name})")
            }
            TabKind::ObjectStoreBucket {
                connection_name,
                bucket_name,
                ..
            } => {
                format!("{bucket_name} ({connection_name})")
            }
            TabKind::ServerInfo {
                connection_name, ..
            } => {
                format!("{} ({})", t("server_info.title"), connection_name)
            }
            TabKind::Metrics {
                connection_name, ..
            } => {
                format!("{} ({})", t("common.tab_metrics"), connection_name)
            }
            TabKind::SearchWorkspace { .. } => t("common.tab_search_workspace").to_string(),
            TabKind::Settings => t("settings.title").to_string(),
            TabKind::LogViewer => t("log_viewer.title").to_string(),
        }
    }

    /// Structural identity for use in close actions and deduplication.
    pub fn tab_id(&self) -> egui::Id {
        match self {
            TabKind::Welcome => egui::Id::new("tab:welcome"),
            TabKind::Publisher {
                connection_id,
                backend_id,
                ..
            } => egui::Id::new(("tab:publisher", *connection_id, *backend_id)),
            TabKind::Subscriber {
                connection_id,
                backend_id,
                ..
            } => egui::Id::new(("tab:subscriber", *connection_id, *backend_id)),
            TabKind::Stream {
                connection_id,
                stream_name,
                ..
            } => egui::Id::new(("tab:stream", *connection_id, stream_name.as_str())),
            TabKind::KvBucket {
                connection_id,
                bucket_name,
                ..
            } => egui::Id::new(("tab:kv", *connection_id, bucket_name.as_str())),
            TabKind::ObjectStoreBucket {
                connection_id,
                bucket_name,
                ..
            } => egui::Id::new(("tab:object-store", *connection_id, bucket_name.as_str())),
            TabKind::ServerInfo { connection_id, .. } => {
                egui::Id::new(("tab:server-info", *connection_id))
            }
            TabKind::Metrics { connection_id, .. } => {
                egui::Id::new(("tab:metrics", *connection_id))
            }
            TabKind::SearchWorkspace { .. } => egui::Id::new("tab:search-workspace"),
            TabKind::Settings => egui::Id::new("tab:settings"),
            TabKind::LogViewer => egui::Id::new("tab:log-viewer"),
        }
    }
}

pub struct AppTabViewer<'a> {
    pub backend: &'a nats_backend::BackendHandle,
    pub actions: &'a mut Vec<TabAction>,
    pub search_sources: &'a [SearchSourceSnapshot],
    pub settings: &'a mut crate::settings::AppSettings,
    pub theme_id: &'a mut ThemeId,
    pub log_buffer: &'a crate::log_layer::SharedLogBuffer,
    pub proto_manager: &'a ProtoSchemaManager,
}

impl TabViewer for AppTabViewer<'_> {
    type Tab = TabKind;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        tab.tab_id()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        crate::tabs::viewer::render_tab(self, ui, tab);
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab, _path: egui_dock::NodePath) {
        let tab_id = self.id(tab);
        if ui.button(t("common.close_others")).clicked() {
            self.actions.push(TabAction::CloseOtherTabs {
                keep_tab_id: tab_id,
            });
            ui.close();
        }
        if ui.button(t("common.close_all")).clicked() {
            self.actions.push(TabAction::CloseAllTabs);
            ui.close();
        }
        if ui.button(t("common.close_to_right")).clicked() {
            self.actions
                .push(TabAction::CloseTabsToRight { of_tab_id: tab_id });
            ui.close();
        }
    }

    fn is_closeable(&self, tab: &Self::Tab) -> bool {
        !matches!(tab, TabKind::Welcome)
    }

    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, TabKind::Welcome)
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        // Send explicit Unsubscribe to clean up worker state.
        // TabGuard Drop handles cancellation + display ID recycling automatically.
        if let TabKind::Subscriber {
            connection_id,
            backend_id,
            state,
            ..
        } = tab
        {
            for sub in &state.subscriptions {
                if sub.active {
                    self.backend
                        .send(nats_backend::BackendCommand::Unsubscribe {
                            connection_id: *connection_id,
                            backend_id: *backend_id,
                            subject: sub.subject.clone(),
                        });
                }
            }
        }
        OnCloseResponse::Close
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use nats_backend::{MetricsSection, MetricsSectionError, MetricsSnapshot, VarzMetrics};

    use super::MetricsState;

    fn sample_snapshot(endpoint: &str, collected_at: SystemTime, in_msgs: u64) -> MetricsSnapshot {
        MetricsSnapshot {
            endpoint: endpoint.to_string(),
            collected_at,
            health: None,
            varz: Some(VarzMetrics {
                server_name: Some("local".to_string()),
                server_id: Some("server-1".to_string()),
                version: Some("2.11.0".to_string()),
                host: Some("127.0.0.1".to_string()),
                port: Some(4222),
                uptime: Some("1m".to_string()),
                mem_bytes: 1024,
                cpu_percent: 1.0,
                connections: 2,
                total_connections: 3,
                subscriptions: 4,
                slow_consumers: 0,
                in_msgs,
                out_msgs: in_msgs,
                in_bytes: 2048,
                out_bytes: 2048,
            }),
            connz: None,
            jsz: None,
            errors: Vec::new(),
        }
    }

    #[test]
    fn metrics_state_marks_last_good_sample_stale_after_failed_refresh() {
        let mut state = MetricsState::with_endpoint("http://localhost:8222".to_string());
        let first = sample_snapshot("http://localhost:8222", SystemTime::UNIX_EPOCH, 10);
        state.apply_snapshot(first);

        state.apply_snapshot(MetricsSnapshot {
            endpoint: "http://localhost:8222".to_string(),
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
            health: None,
            varz: None,
            connz: None,
            jsz: None,
            errors: vec![MetricsSectionError {
                section: MetricsSection::Varz,
                message: "timeout".to_string(),
            }],
        });

        assert!(state.is_stale());
        assert_eq!(state.history().len(), 1);
        assert_eq!(
            state.latest_data().unwrap().varz.as_ref().unwrap().in_msgs,
            10
        );
    }

    #[test]
    fn metrics_state_clears_history_when_endpoint_changes() {
        let mut state = MetricsState::with_endpoint("http://localhost:8222".to_string());
        state.apply_snapshot(sample_snapshot(
            "http://localhost:8222",
            SystemTime::UNIX_EPOCH,
            10,
        ));

        state.set_endpoint("http://localhost:9222".to_string());

        assert!(state.history().is_empty());
        assert!(state.latest_attempt().is_none());
        assert_eq!(state.endpoint(), "http://localhost:9222");
    }

    #[test]
    fn metrics_state_defaults_to_enabled_auto_refresh_every_five_seconds() {
        let state = MetricsState::with_endpoint("http://localhost:8222".to_string());

        assert!(state.auto_refresh.enabled);
        assert_eq!(state.auto_refresh.interval_secs, 5);
    }
}
