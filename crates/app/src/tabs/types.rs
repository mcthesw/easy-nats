use std::collections::VecDeque;
use std::time::{Instant, SystemTime};

use eframe::egui;
use egui_dock::TabViewer;
use egui_dock::tab_viewer::OnCloseResponse;

use crate::format::PayloadFormat;
use crate::i18n::t;
use crate::proto::{ProtoSchemaManager, ProtoViewState};

use super::super::app::TabIdAllocator;

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
        keep_title: String,
    },
    CloseAllTabs,
    CloseTabsToRight {
        of_title: String,
    },
    OpenConnectionEditor,
    ApplyTheme {
        dark: bool,
    },
    LoadProtoSchemas {
        dir: String,
    },
    ClearProtoSchemas,
}

#[derive(Debug)]
pub struct PublisherState {
    pub subject: String,
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
    pub subject: String,
    pub reply: Option<String>,
    pub headers: Vec<(String, String)>,
    pub payload: Vec<u8>,
    pub timestamp: SystemTime,
}

#[derive(Debug)]
pub struct SubjectSubscription {
    pub subject: String,
    pub active: bool,
}

#[derive(Debug)]
pub struct SubscriberState {
    pub subject_input: String,
    pub subscriptions: Vec<SubjectSubscription>,
    pub messages: VecDeque<ReceivedMessage>,
    pub max_messages: usize,
    pub selected_idx: Option<usize>,
    pub payload_format: PayloadFormat,
    /// When set, only display messages matching this subject.
    pub subject_filter: Option<String>,
    pub proto_view: ProtoViewState,
}

impl Default for SubscriberState {
    fn default() -> Self {
        Self {
            subject_input: String::new(),
            subscriptions: Vec::new(),
            messages: VecDeque::new(),
            max_messages: 1000,
            selected_idx: None,
            payload_format: PayloadFormat::Auto,
            subject_filter: None,
            proto_view: ProtoViewState::default(),
        }
    }
}

impl SubscriberState {
    pub fn push_message(&mut self, msg: ReceivedMessage) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
            if let Some(idx) = self.selected_idx {
                self.selected_idx = if idx == 0 { None } else { Some(idx - 1) };
            }
        }
        self.messages.push_back(msg);
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
        }
    }
}

#[derive(Debug)]
pub struct KvBucketState {
    pub info: Option<serde_json::Value>,
    pub entries: Vec<serde_json::Value>,
    pub selected_key: Option<String>,
    pub key_filter: String,
    pub loading_entries: bool,
    pub loading_history: bool,
    pub entry_key: String,
    pub entry_value: String,
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
            entries: Vec::new(),
            selected_key: None,
            key_filter: String::new(),
            loading_entries: false,
            loading_history: false,
            entry_key: String::new(),
            entry_value: String::new(),
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

#[derive(Debug)]
pub enum TabKind {
    Welcome,
    Publisher {
        connection_id: u64,
        connection_name: String,
        instance_id: u32,
        state: PublisherState,
    },
    Subscriber {
        connection_id: u64,
        connection_name: String,
        instance_id: u32,
        state: SubscriberState,
    },
    Stream {
        connection_id: u64,
        connection_name: String,
        stream_name: String,
        state: StreamState,
    },
    KvBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
        state: KvBucketState,
    },
    ObjectStoreBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
        state: ObjectStoreBucketState,
    },
    ServerInfo {
        connection_id: u64,
        connection_name: String,
        state: ServerInfoState,
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
                instance_id,
                ..
            } => {
                format!(
                    "{} #{} ({})",
                    t("common.tab_publisher"),
                    instance_id,
                    connection_name
                )
            }
            TabKind::Subscriber {
                connection_name,
                instance_id,
                ..
            } => {
                format!(
                    "{} #{} ({})",
                    t("common.tab_subscriber"),
                    instance_id,
                    connection_name
                )
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
            TabKind::Settings => t("settings.title").to_string(),
            TabKind::LogViewer => t("log_viewer.title").to_string(),
        }
    }
}

pub struct AppTabViewer<'a> {
    pub backend: &'a nats_backend::BackendHandle,
    pub actions: &'a mut Vec<TabAction>,
    pub settings: &'a mut crate::settings::AppSettings,
    pub dark_mode: &'a mut bool,
    pub log_buffer: &'a crate::log_layer::SharedLogBuffer,
    pub proto_manager: &'a ProtoSchemaManager,
    pub tab_id_alloc: &'a mut TabIdAllocator,
}

impl TabViewer for AppTabViewer<'_> {
    type Tab = TabKind;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match tab {
            TabKind::Welcome => egui::Id::new("tab:welcome"),
            TabKind::Publisher {
                connection_id,
                instance_id,
                ..
            } => egui::Id::new(("tab:publisher", connection_id, instance_id)),
            TabKind::Subscriber {
                connection_id,
                instance_id,
                ..
            } => egui::Id::new(("tab:subscriber", connection_id, instance_id)),
            TabKind::Stream {
                connection_id,
                stream_name,
                ..
            } => egui::Id::new(("tab:stream", connection_id, stream_name)),
            TabKind::KvBucket {
                connection_id,
                bucket_name,
                ..
            } => egui::Id::new(("tab:kv", connection_id, bucket_name)),
            TabKind::ObjectStoreBucket {
                connection_id,
                bucket_name,
                ..
            } => egui::Id::new(("tab:object-store", connection_id, bucket_name)),
            TabKind::ServerInfo { connection_id, .. } => {
                egui::Id::new(("tab:server-info", connection_id))
            }
            TabKind::Settings => egui::Id::new("tab:settings"),
            TabKind::LogViewer => egui::Id::new("tab:log-viewer"),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        crate::tabs::viewer::render_tab(self, ui, tab);
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab, _path: egui_dock::NodePath) {
        let title = tab.title();
        if ui.button(t("common.close_others")).clicked() {
            self.actions.push(TabAction::CloseOtherTabs {
                keep_title: title.clone(),
            });
            ui.close();
        }
        if ui.button(t("common.close_all")).clicked() {
            self.actions.push(TabAction::CloseAllTabs);
            ui.close();
        }
        if ui.button(t("common.close_to_right")).clicked() {
            self.actions
                .push(TabAction::CloseTabsToRight { of_title: title });
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
        match tab {
            TabKind::Subscriber {
                connection_id,
                instance_id,
                state,
                ..
            } => {
                for sub in &state.subscriptions {
                    if sub.active {
                        self.backend
                            .send(nats_backend::BackendCommand::Unsubscribe {
                                connection_id: *connection_id,
                                subscriber_id: *instance_id,
                                subject: sub.subject.clone(),
                            });
                    }
                }
                self.tab_id_alloc.free(*instance_id);
            }
            TabKind::Publisher { instance_id, .. } => {
                self.tab_id_alloc.free(*instance_id);
            }
            _ => {}
        }
        OnCloseResponse::Close
    }
}
