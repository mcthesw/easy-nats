use std::collections::VecDeque;
use std::time::SystemTime;

use eframe::egui;
use egui_dock::TabViewer;

use crate::format::PayloadFormat;
use crate::ui_strings;

#[derive(Debug, Clone)]
pub enum TabAction {
    OpenConsumerCreate {
        connection_id: u64,
        stream_name: String,
    },
    ConfirmDeleteKvBucket {
        connection_id: u64,
        bucket_name: String,
    },
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
pub struct SubscriberState {
    pub subject: String,
    pub subscribed: bool,
    pub messages: VecDeque<ReceivedMessage>,
    pub max_messages: usize,
    pub selected_idx: Option<usize>,
    pub payload_format: PayloadFormat,
}

impl Default for SubscriberState {
    fn default() -> Self {
        Self {
            subject: String::new(),
            subscribed: false,
            messages: VecDeque::new(),
            max_messages: 1000,
            selected_idx: None,
            payload_format: PayloadFormat::Auto,
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
    pub batch_size: String,
    pub fetching: bool,
    pub purge_subject: String,
    pub consumers: Vec<serde_json::Value>,
    pub consumers_fetching: bool,
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
            batch_size: "50".to_string(),
            fetching: false,
            purge_subject: String::new(),
            consumers: Vec::new(),
            consumers_fetching: false,
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
        }
    }
}

#[derive(Debug)]
pub enum TabKind {
    Welcome,
    Publisher {
        connection_id: u64,
        connection_name: String,
        state: PublisherState,
    },
    Subscriber {
        connection_id: u64,
        connection_name: String,
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
    #[allow(dead_code)]
    ObjectStoreBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
    },
}

impl TabKind {
    pub fn title(&self) -> String {
        match self {
            TabKind::Welcome => ui_strings::TAB_WELCOME.to_string(),
            TabKind::Publisher {
                connection_name, ..
            } => {
                format!("{} ({})", ui_strings::TAB_PUBLISHER, connection_name)
            }
            TabKind::Subscriber {
                connection_name, ..
            } => {
                format!("{} ({})", ui_strings::TAB_SUBSCRIBER, connection_name)
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
        }
    }
}

pub struct AppTabViewer<'a> {
    pub backend: &'a nats_backend::BackendHandle,
    pub actions: &'a mut Vec<TabAction>,
}

impl TabViewer for AppTabViewer<'_> {
    type Tab = TabKind;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        crate::tabs::viewer::render_tab(self, ui, tab);
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, TabKind::Welcome)
    }
}
