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
            TabKind::Subscriber { .. } => {
                ui.label("Subscriber tab — coming soon");
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
