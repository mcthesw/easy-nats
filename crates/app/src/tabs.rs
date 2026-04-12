use std::collections::VecDeque;
use std::time::SystemTime;

use eframe::egui;
use egui::WidgetText;
use egui_dock::TabViewer;
use nats_backend::{BackendCommand, BackendHandle};

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
}

impl Default for SubscriberState {
    fn default() -> Self {
        Self {
            subject: String::new(),
            subscribed: false,
            messages: VecDeque::new(),
            max_messages: 1000,
            selected_idx: None,
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
            TabKind::Stream { stream_name, .. } => {
                ui.label(format!("Stream: {stream_name} — coming soon"));
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
    ui.label(S::PUBLISHER_RESPONSE);
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
        let text = String::from_utf8_lossy(&resp.payload);
        egui::ScrollArea::vertical()
            .id_salt("resp_payload")
            .max_height(200.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut text.to_string())
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .code_editor(),
                );
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
        message_detail_ui(ui, msg);
    } else {
        ui.label(S::SUBSCRIBER_SELECT_MSG);
    }
}

fn message_detail_ui(ui: &mut egui::Ui, msg: &ReceivedMessage) {
    ui.label(S::SUBSCRIBER_DETAIL);
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
    let text = String::from_utf8_lossy(&msg.payload);
    egui::ScrollArea::vertical()
        .id_salt("msg_detail_payload")
        .max_height(200.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text.to_string())
                    .desired_width(f32::INFINITY)
                    .desired_rows(4)
                    .code_editor(),
            );
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
