use std::collections::VecDeque;
use std::time::SystemTime;

use base64::Engine;
use eframe::egui;
use egui::WidgetText;
use egui_dock::TabViewer;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format::{self, PayloadFormat};
use crate::ui_strings;
use crate::ui_strings as S;

/// State for a Publisher tab.
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

/// Response data from a request-reply operation.
#[derive(Debug, Clone)]
pub struct ResponseData {
    pub payload: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

/// A single received NATS message.
#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    pub subject: String,
    pub reply: Option<String>,
    pub headers: Vec<(String, String)>,
    pub payload: Vec<u8>,
    pub timestamp: SystemTime,
}

/// State for a Subscriber tab.
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
    /// Push a message into the ring buffer, evicting the oldest if at capacity.
    pub fn push_message(&mut self, msg: ReceivedMessage) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
            // Adjust selected index after eviction
            if let Some(idx) = self.selected_idx {
                if idx == 0 {
                    self.selected_idx = None;
                } else {
                    self.selected_idx = Some(idx - 1);
                }
            }
        }
        self.messages.push_back(msg);
    }
}

/// State for a Stream tab.
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
        }
    }
}

/// Represents tabs in the dock area.
#[derive(Debug)]
#[allow(dead_code)]
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
    },
    ObjectStoreBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
    },
}

impl TabKind {
    /// Format: "ResourceName (ServerName)"
    pub fn title(&self) -> String {
        match self {
            TabKind::Welcome => ui_strings::TAB_WELCOME.to_string(),
            TabKind::Publisher {
                connection_name, ..
            } => format!("{} ({})", ui_strings::TAB_PUBLISHER, connection_name),
            TabKind::Subscriber {
                connection_name, ..
            } => format!("{} ({})", ui_strings::TAB_SUBSCRIBER, connection_name),
            TabKind::Stream {
                connection_name,
                stream_name,
                ..
            } => format!("{stream_name} ({connection_name})"),
            TabKind::KvBucket {
                connection_name,
                bucket_name,
                ..
            } => format!("{bucket_name} ({connection_name})"),
            TabKind::ObjectStoreBucket {
                connection_name,
                bucket_name,
                ..
            } => format!("{bucket_name} ({connection_name})"),
        }
    }
}

/// Viewer that renders each tab type's content.
pub struct AppTabViewer<'a> {
    pub backend: &'a BackendHandle,
}

impl TabViewer for AppTabViewer<'_> {
    type Tab = TabKind;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            TabKind::Welcome => {
                ui.heading(ui_strings::WELCOME_HEADING);
                ui.label(ui_strings::WELCOME_BODY);
            }
            TabKind::Publisher {
                connection_id,
                state,
                ..
            } => {
                let conn_id = *connection_id;
                publisher_ui(ui, conn_id, state, self.backend);
            }
            TabKind::Subscriber {
                connection_id,
                state,
                ..
            } => {
                let conn_id = *connection_id;
                subscriber_ui(ui, conn_id, state, self.backend);
            }
            TabKind::Stream {
                connection_id,
                stream_name,
                state,
                ..
            } => {
                let conn_id = *connection_id;
                stream_ui(ui, conn_id, stream_name, state, self.backend);
            }
            TabKind::KvBucket { bucket_name, .. } => {
                ui.label(format!("KV Bucket: {bucket_name} — coming soon"));
            }
            TabKind::ObjectStoreBucket { bucket_name, .. } => {
                ui.label(format!("Object Store: {bucket_name} — coming soon"));
            }
        }
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, TabKind::Welcome)
    }
}

fn publisher_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut PublisherState,
    backend: &BackendHandle,
) {
    // Subject
    ui.horizontal(|ui| {
        ui.label(S::PUBLISHER_SUBJECT);
        ui.text_edit_singleline(&mut state.subject);
    });

    ui.add_space(4.0);

    // Headers section
    egui::CollapsingHeader::new(S::PUBLISHER_HEADERS)
        .id_salt("publisher_headers")
        .show(ui, |ui| {
            let mut remove_idx = None;
            for (idx, (key, val)) in state.headers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(key)
                            .hint_text(S::PUBLISHER_HEADER_KEY)
                            .desired_width(120.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(val)
                            .hint_text(S::PUBLISHER_HEADER_VALUE)
                            .desired_width(200.0),
                    );
                    if ui.small_button("✕").clicked() {
                        remove_idx = Some(idx);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                state.headers.remove(idx);
            }
            if ui.small_button(S::PUBLISHER_ADD_HEADER).clicked() {
                state.headers.push((String::new(), String::new()));
            }
        });

    ui.add_space(4.0);

    // Payload
    ui.label(S::PUBLISHER_PAYLOAD);
    egui::ScrollArea::vertical()
        .id_salt("publisher_payload")
        .max_height(200.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.payload)
                    .desired_width(f32::INFINITY)
                    .desired_rows(6)
                    .code_editor(),
            );
        });

    ui.add_space(4.0);

    // Action buttons
    ui.horizontal(|ui| {
        let can_send = !state.subject.trim().is_empty();

        if ui
            .add_enabled(can_send, egui::Button::new(S::PUBLISHER_PUBLISH))
            .clicked()
        {
            let headers = collect_headers(&state.headers);
            backend.send(BackendCommand::Publish {
                connection_id,
                subject: state.subject.clone(),
                payload: state.payload.as_bytes().to_vec(),
                headers,
            });
        }

        ui.separator();

        ui.label(S::PUBLISHER_TIMEOUT);
        ui.add(egui::TextEdit::singleline(&mut state.timeout_ms).desired_width(60.0));

        if ui
            .add_enabled(
                can_send && !state.waiting,
                egui::Button::new(S::PUBLISHER_REQUEST),
            )
            .clicked()
        {
            let headers = collect_headers(&state.headers);
            let timeout_ms = state.timeout_ms.parse::<u64>().unwrap_or(5000);
            backend.send(BackendCommand::Request {
                connection_id,
                subject: state.subject.clone(),
                payload: state.payload.as_bytes().to_vec(),
                headers,
                timeout_ms,
            });
            state.response = None;
            state.waiting = true;
        }
    });

    ui.add_space(8.0);
    ui.separator();

    // Response area
    ui.horizontal(|ui| {
        ui.label(S::PUBLISHER_RESPONSE);
        format::format_selector(ui, "pub_resp_fmt", &mut state.response_format);
    });
    if state.waiting {
        ui.spinner();
        ui.label(S::PUBLISHER_WAITING);
    } else if let Some(resp) = &state.response {
        if !resp.headers.is_empty() {
            ui.label(S::PUBLISHER_RESPONSE_HEADERS);
            egui::Grid::new("resp_headers")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    for (k, v) in &resp.headers {
                        ui.label(k);
                        ui.label(v);
                        ui.end_row();
                    }
                });
            ui.add_space(4.0);
        }
        ui.label(S::PUBLISHER_RESPONSE_PAYLOAD);
        egui::ScrollArea::vertical()
            .id_salt("resp_payload")
            .max_height(200.0)
            .show(ui, |ui| {
                format::render_payload(ui, &resp.payload, state.response_format);
            });
    } else {
        ui.label(S::PUBLISHER_NO_RESPONSE);
    }
}

fn collect_headers(headers: &[(String, String)]) -> Option<Vec<(String, String)>> {
    let non_empty: Vec<(String, String)> = headers
        .iter()
        .filter(|(k, v)| !k.trim().is_empty() || !v.trim().is_empty())
        .cloned()
        .collect();
    if non_empty.is_empty() {
        None
    } else {
        Some(non_empty)
    }
}

fn subscriber_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut SubscriberState,
    backend: &BackendHandle,
) {
    // Subject input + subscribe/unsubscribe toggle
    ui.horizontal(|ui| {
        ui.label(S::SUBSCRIBER_SUBJECT);
        ui.add_enabled(
            !state.subscribed,
            egui::TextEdit::singleline(&mut state.subject),
        );
        let can_toggle = !state.subject.trim().is_empty();
        if state.subscribed {
            if ui
                .add_enabled(true, egui::Button::new(S::SUBSCRIBER_UNSUBSCRIBE))
                .clicked()
            {
                backend.send(BackendCommand::Unsubscribe {
                    connection_id,
                    subject: state.subject.clone(),
                });
                state.subscribed = false;
            }
        } else if ui
            .add_enabled(can_toggle, egui::Button::new(S::SUBSCRIBER_SUBSCRIBE))
            .clicked()
        {
            backend.send(BackendCommand::Subscribe {
                connection_id,
                subject: state.subject.clone(),
            });
            state.subscribed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label(format!(
            "{} {} / {}",
            S::SUBSCRIBER_MSG_COUNT,
            state.messages.len(),
            state.max_messages
        ));
        if ui.small_button(S::SUBSCRIBER_CLEAR).clicked() {
            state.messages.clear();
            state.selected_idx = None;
        }
    });

    ui.add_space(4.0);
    ui.separator();

    // Split: message list on top, detail on bottom
    let available = ui.available_height();
    let list_height = (available * 0.5).max(100.0);

    // ── Message list ──
    ui.label(S::SUBSCRIBER_MESSAGES);
    egui::ScrollArea::vertical()
        .id_salt("sub_msg_list")
        .max_height(list_height)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if state.messages.is_empty() {
                ui.label(S::SUBSCRIBER_NO_MESSAGES);
            } else {
                for (idx, msg) in state.messages.iter().enumerate() {
                    let ts = format_timestamp(msg.timestamp);
                    let preview = payload_preview(&msg.payload, 80);
                    let label = format!("[{ts}] {} — {preview}", msg.subject);
                    let selected = state.selected_idx == Some(idx);
                    if ui.selectable_label(selected, label).clicked() {
                        state.selected_idx = Some(idx);
                    }
                }
            }
        });

    ui.add_space(4.0);
    ui.separator();

    // ── Message detail panel ──
    if let Some(idx) = state.selected_idx
        && let Some(msg) = state.messages.get(idx)
    {
        message_detail_ui(ui, msg, &mut state.payload_format);
    } else {
        ui.label(S::SUBSCRIBER_SELECT_MSG);
    }
}

fn message_detail_ui(ui: &mut egui::Ui, msg: &ReceivedMessage, payload_format: &mut PayloadFormat) {
    ui.horizontal(|ui| {
        ui.label(S::SUBSCRIBER_DETAIL);
        format::format_selector(ui, "sub_detail_fmt", payload_format);
    });
    ui.add_space(2.0);

    egui::Grid::new("msg_detail_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(S::SUBSCRIBER_DETAIL_SUBJECT);
            ui.label(&msg.subject);
            ui.end_row();

            if let Some(reply) = &msg.reply {
                ui.label(S::SUBSCRIBER_DETAIL_REPLY);
                ui.label(reply);
                ui.end_row();
            }

            ui.label(S::SUBSCRIBER_DETAIL_TIMESTAMP);
            ui.label(format_timestamp(msg.timestamp));
            ui.end_row();

            ui.label(S::SUBSCRIBER_DETAIL_SIZE);
            ui.label(format!("{} bytes", msg.payload.len()));
            ui.end_row();
        });

    if !msg.headers.is_empty() {
        ui.add_space(4.0);
        ui.label(S::SUBSCRIBER_DETAIL_HEADERS);
        egui::Grid::new("msg_detail_headers")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for (k, v) in &msg.headers {
                    ui.label(k);
                    ui.label(v);
                    ui.end_row();
                }
            });
    }

    ui.add_space(4.0);
    ui.label(S::SUBSCRIBER_DETAIL_PAYLOAD);
    egui::ScrollArea::vertical()
        .id_salt("msg_detail_payload")
        .max_height(200.0)
        .show(ui, |ui| {
            format::render_payload(ui, &msg.payload, *payload_format);
        });
}

fn format_timestamp(ts: SystemTime) -> String {
    match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => {
            let total_secs = d.as_secs();
            let hours = (total_secs / 3600) % 24;
            let minutes = (total_secs / 60) % 60;
            let seconds = total_secs % 60;
            let millis = d.subsec_millis();
            format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
        }
        Err(_) => "??:??:??".to_string(),
    }
}

fn payload_preview(payload: &[u8], max_len: usize) -> String {
    let s = String::from_utf8_lossy(payload);
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

// ─── Stream tab UI ───

fn stream_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    state: &mut StreamState,
    backend: &BackendHandle,
) {
    // Stream info panel
    if let Some(info) = &state.info {
        egui::CollapsingHeader::new(S::STREAM_INFO)
            .id_salt("stream_info")
            .default_open(true)
            .show(ui, |ui| {
                stream_info_panel(ui, info);
            });
        ui.separator();
    }

    // Message browser controls
    ui.horizontal(|ui| {
        ui.label(S::STREAM_START_SEQ);
        ui.add(egui::TextEdit::singleline(&mut state.start_seq).desired_width(80.0));
        ui.label(S::STREAM_SUBJECT_FILTER);
        ui.add(egui::TextEdit::singleline(&mut state.subject_filter).desired_width(120.0));
        ui.label(S::STREAM_BATCH_SIZE);
        ui.add(egui::TextEdit::singleline(&mut state.batch_size).desired_width(60.0));
        if ui
            .add_enabled(!state.fetching, egui::Button::new(S::STREAM_FETCH))
            .clicked()
        {
            let start = state.start_seq.parse().ok();
            let filter = if state.subject_filter.is_empty() {
                None
            } else {
                Some(state.subject_filter.clone())
            };
            let batch = state.batch_size.parse::<u64>().unwrap_or(50);
            backend.send(BackendCommand::GetStreamMessages {
                connection_id,
                stream: stream_name.to_string(),
                start_sequence: start,
                subject_filter: filter,
                batch_size: batch,
            });
            state.fetching = true;
        }
    });

    if state.fetching {
        ui.spinner();
    }

    ui.add_space(4.0);

    // Message list
    let available = ui.available_height();
    let list_height = (available * 0.45).max(100.0);

    ui.label(S::STREAM_MESSAGES);
    egui::ScrollArea::vertical()
        .id_salt("stream_msg_list")
        .max_height(list_height)
        .show(ui, |ui| {
            if state.messages.is_empty() {
                ui.label(S::STREAM_NO_MESSAGES);
            } else {
                for (idx, msg) in state.messages.iter().enumerate() {
                    let seq = msg["sequence"].as_u64().unwrap_or(0);
                    let subject = msg["subject"].as_str().unwrap_or("");
                    let label = format!("#{seq} — {subject}");
                    let selected = state.selected_msg == Some(idx);
                    if ui.selectable_label(selected, label).clicked() {
                        state.selected_msg = Some(idx);
                    }
                }
            }
        });

    ui.separator();

    // Message detail
    if let Some(idx) = state.selected_msg {
        if let Some(msg) = state.messages.get(idx) {
            stream_message_detail(ui, msg, &mut state.payload_format);
        }
    } else {
        ui.label(S::STREAM_SELECT_MSG);
    }

    ui.separator();

    // Purge controls
    egui::CollapsingHeader::new(S::STREAM_PURGE)
        .id_salt("stream_purge")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(S::STREAM_PURGE_SUBJECT);
                ui.text_edit_singleline(&mut state.purge_subject);
            });
            ui.horizontal(|ui| {
                if ui.button(S::STREAM_PURGE_FILTERED).clicked() && !state.purge_subject.is_empty()
                {
                    backend.send(BackendCommand::PurgeStream {
                        connection_id,
                        name: stream_name.to_string(),
                        filter: Some(state.purge_subject.clone()),
                    });
                }
                if ui.button(S::STREAM_PURGE_ALL).clicked() {
                    backend.send(BackendCommand::PurgeStream {
                        connection_id,
                        name: stream_name.to_string(),
                        filter: None,
                    });
                }
            });
            // Delete individual message
            if let Some(idx) = state.selected_msg
                && let Some(msg) = state.messages.get(idx)
                && let Some(seq) = msg["sequence"].as_u64()
                && ui
                    .button(format!("{} #{seq}", S::STREAM_DELETE_MSG))
                    .clicked()
            {
                backend.send(BackendCommand::DeleteStreamMessage {
                    connection_id,
                    stream: stream_name.to_string(),
                    sequence: seq,
                });
            }
        });
}

fn stream_info_panel(ui: &mut egui::Ui, info: &serde_json::Value) {
    egui::Grid::new("stream_info_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            if let Some(config) = info.get("config") {
                if let Some(name) = config.get("name").and_then(|v| v.as_str()) {
                    ui.label(S::STREAM_NAME);
                    ui.label(name);
                    ui.end_row();
                }
                if let Some(subjects) = config.get("subjects").and_then(|v| v.as_array()) {
                    ui.label(S::STREAM_SUBJECTS);
                    let subj_str: String = subjects
                        .iter()
                        .filter_map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    ui.label(subj_str);
                    ui.end_row();
                }
                if let Some(storage) = config.get("storage").and_then(|v| v.as_str()) {
                    ui.label(S::STREAM_STORAGE);
                    ui.label(storage);
                    ui.end_row();
                }
                if let Some(retention) = config.get("retention").and_then(|v| v.as_str()) {
                    ui.label(S::STREAM_RETENTION);
                    ui.label(retention);
                    ui.end_row();
                }
            }
            if let Some(st) = info.get("state") {
                if let Some(msgs) = st.get("messages").and_then(|v| v.as_u64()) {
                    ui.label(S::STREAM_MSG_COUNT);
                    ui.label(msgs.to_string());
                    ui.end_row();
                }
                if let Some(bytes) = st.get("bytes").and_then(|v| v.as_u64()) {
                    ui.label(S::STREAM_BYTES);
                    ui.label(format_bytes(bytes));
                    ui.end_row();
                }
                if let Some(consumers) = st.get("consumer_count").and_then(|v| v.as_u64()) {
                    ui.label(S::STREAM_CONSUMERS);
                    ui.label(consumers.to_string());
                    ui.end_row();
                }
            }
        });
}

fn stream_message_detail(
    ui: &mut egui::Ui,
    msg: &serde_json::Value,
    payload_format: &mut PayloadFormat,
) {
    ui.horizontal(|ui| {
        ui.label(S::STREAM_MSG_DETAIL);
        format::format_selector(ui, "stream_msg_fmt", payload_format);
    });

    egui::Grid::new("stream_msg_detail_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            if let Some(seq) = msg["sequence"].as_u64() {
                ui.label(S::STREAM_MSG_SEQUENCE);
                ui.label(seq.to_string());
                ui.end_row();
            }
            if let Some(subject) = msg["subject"].as_str() {
                ui.label(S::STREAM_MSG_SUBJECT);
                ui.label(subject);
                ui.end_row();
            }
        });

    // Headers
    if let Some(headers) = msg["headers"].as_array()
        && !headers.is_empty()
    {
        ui.add_space(4.0);
        ui.label(S::STREAM_MSG_HEADERS);
        egui::Grid::new("stream_msg_headers")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for h in headers {
                    if let Some(arr) = h.as_array()
                        && arr.len() == 2
                    {
                        let k = arr[0].as_str().unwrap_or("");
                        let v = arr[1].as_str().unwrap_or("");
                        ui.label(k);
                        ui.label(v);
                        ui.end_row();
                    }
                }
            });
    }

    // Payload
    ui.add_space(4.0);
    ui.label(S::STREAM_MSG_PAYLOAD);
    egui::ScrollArea::vertical()
        .id_salt("stream_msg_payload")
        .max_height(200.0)
        .show(ui, |ui| {
            if let Some(payload_b64) = msg["payload_base64"].as_str() {
                match base64::engine::general_purpose::STANDARD.decode(payload_b64) {
                    Ok(data) => format::render_payload(ui, &data, *payload_format),
                    Err(_) => {
                        ui.label("Invalid base64 payload");
                    }
                }
            } else {
                ui.label("No payload");
            }
        });
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(subject: &str) -> ReceivedMessage {
        ReceivedMessage {
            subject: subject.to_string(),
            reply: None,
            headers: Vec::new(),
            payload: Vec::new(),
            timestamp: SystemTime::now(),
        }
    }

    #[test]
    fn ring_buffer_appends_within_capacity() {
        let mut state = SubscriberState {
            max_messages: 3,
            ..Default::default()
        };
        state.push_message(make_msg("a"));
        state.push_message(make_msg("b"));
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].subject, "a");
        assert_eq!(state.messages[1].subject, "b");
    }

    #[test]
    fn ring_buffer_evicts_oldest_at_capacity() {
        let mut state = SubscriberState {
            max_messages: 2,
            ..Default::default()
        };
        state.push_message(make_msg("a"));
        state.push_message(make_msg("b"));
        state.push_message(make_msg("c"));
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].subject, "b");
        assert_eq!(state.messages[1].subject, "c");
    }

    #[test]
    fn ring_buffer_adjusts_selected_idx_on_eviction() {
        let mut state = SubscriberState {
            max_messages: 2,
            ..Default::default()
        };
        state.push_message(make_msg("a"));
        state.push_message(make_msg("b"));
        state.selected_idx = Some(1); // selecting "b"
        state.push_message(make_msg("c")); // evicts "a", "b" shifts to idx 0
        assert_eq!(state.selected_idx, Some(0));
    }

    #[test]
    fn ring_buffer_clears_selected_idx_when_evicted() {
        let mut state = SubscriberState {
            max_messages: 2,
            ..Default::default()
        };
        state.push_message(make_msg("a"));
        state.push_message(make_msg("b"));
        state.selected_idx = Some(0); // selecting "a"
        state.push_message(make_msg("c")); // evicts "a"
        assert_eq!(state.selected_idx, None);
    }

    #[test]
    fn payload_preview_truncates() {
        assert_eq!(payload_preview(b"hello", 10), "hello");
        assert_eq!(payload_preview(b"hello world!!", 5), "hello…");
    }
}
