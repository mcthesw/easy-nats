use base64::Engine;
use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};
use std::time::{Duration, SystemTime};

use crate::format::{self, PayloadFormat};
use crate::i18n::t;
use crate::proto::ProtoSchemaManager;

use super::common::{auto_refresh_ui, format_bytes};
use super::stream_consumers::render_consumers;
use super::types::{StreamState, TabAction};

pub fn stream_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    state: &mut StreamState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
    proto_manager: &ProtoSchemaManager,
) {
    // Auto-refresh toggle
    ui.horizontal(|ui| {
        auto_refresh_ui(ui, "stream_auto_refresh", &mut state.auto_refresh);
    });

    // Auto-refresh timer: refresh consumers list
    if state.auto_refresh.should_refresh() {
        backend.send(BackendCommand::ListConsumers {
            connection_id,
            stream: stream_name.to_string(),
        });
        state.auto_refresh.mark_refreshed();
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
    } else if state.auto_refresh.enabled {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
    }

    // Collapsible stream info header
    if let Some(info) = &state.info {
        egui::CollapsingHeader::new(t("stream.info"))
            .id_salt("stream_info")
            .default_open(false)
            .show(ui, |ui| stream_info_panel(ui, info));
    }

    // Message browser controls
    render_message_controls(ui, connection_id, stream_name, state, backend, actions);
    ui.separator();

    // Horizontal split: left message list, right message detail + consumers + purge
    let panel_id = egui::Id::new(("stream_left_panel", connection_id, stream_name));
    egui::Panel::left(panel_id)
        .resizable(true)
        .default_size(300.0)
        .size_range(200.0..=f32::INFINITY)
        .show_inside(ui, |ui| {
            render_message_list(ui, state);
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        // Bottom: consumers + purge (resizable, scrollable to avoid overlap)
        egui::Panel::bottom(egui::Id::new((
            "stream_right_bottom",
            connection_id,
            stream_name,
        )))
        .resizable(true)
        .default_size(150.0)
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(("stream_bottom_scroll", connection_id, stream_name))
                .auto_shrink(false)
                .show(ui, |ui| {
                    render_consumers(ui, connection_id, stream_name, state, backend, actions);
                    ui.separator();
                    render_purge_controls(ui, connection_id, stream_name, state, backend);
                });
        });

        egui::ScrollArea::vertical()
            .id_salt(("stream_detail_scroll", connection_id, stream_name))
            .auto_shrink(false)
            .show(ui, |ui| {
                if let Some(idx) = state.selected_msg {
                    if let Some(msg) = state.messages.get(idx) {
                        stream_message_detail(
                            ui,
                            msg,
                            &mut state.payload_format,
                            &mut state.proto_view,
                            proto_manager,
                        );
                    }
                } else {
                    ui.label(t("stream.select_msg"));
                }
            });
    });
}

fn render_message_controls(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    state: &mut StreamState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
) {
    // WorkQueue retention warning
    if let Some(info) = &state.info {
        let retention = info["config"]["retention"]
            .as_str()
            .unwrap_or("")
            .to_lowercase();
        if retention.contains("work") {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::YELLOW, "⚠");
                ui.label(t("stream.workqueue_hint"));
            });
        }
    }

    ui.horizontal(|ui| {
        ui.label(t("stream.start_seq"));
        ui.add(egui::TextEdit::singleline(&mut state.start_seq).desired_width(80.0));
        ui.label(t("stream.subject_filter"));
        ui.add(egui::TextEdit::singleline(&mut state.subject_filter).desired_width(120.0));
        ui.label(t("stream.batch_size"));
        ui.add(egui::TextEdit::singleline(&mut state.batch_size).desired_width(60.0));
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(t("stream.start_time"));
        ui.add(
            egui::TextEdit::singleline(&mut state.start_time)
                .desired_width(220.0)
                .hint_text("2025-01-01T00:00:00Z"),
        );
        for (label_key, secs) in [
            ("stream.time_1h", 3600u64),
            ("stream.time_24h", 86400),
            ("stream.time_7d", 604800),
            ("stream.time_30d", 2592000),
        ] {
            if ui.small_button(t(label_key)).clicked() {
                state.start_time =
                    system_time_to_rfc3339(SystemTime::now() - Duration::from_secs(secs));
            }
        }
        if !state.start_time.is_empty() && ui.small_button(t("stream.time_clear")).clicked() {
            state.start_time.clear();
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("stream.publish_message")).clicked() {
            actions.push(TabAction::OpenStreamPublish {
                connection_id,
                stream_name: stream_name.to_string(),
                subject: default_publish_subject(state),
            });
        }
        if ui
            .add_enabled(!state.fetching, egui::Button::new(t("stream.fetch")))
            .clicked()
        {
            let start_time = if state.start_time.trim().is_empty() {
                None
            } else {
                Some(state.start_time.trim().to_string())
            };
            backend.send(BackendCommand::GetStreamMessages {
                connection_id,
                stream: stream_name.to_string(),
                start_sequence: state.start_seq.parse().ok(),
                subject_filter: (!state.subject_filter.is_empty())
                    .then(|| state.subject_filter.clone()),
                start_time,
                batch_size: state.batch_size.parse::<u64>().unwrap_or(50),
            });
            state.fetching = true;
        }
    });

    if state.fetching {
        ui.spinner();
    }
}

fn render_message_list(ui: &mut egui::Ui, state: &mut StreamState) {
    ui.label(t("stream.messages"));
    egui::ScrollArea::vertical()
        .id_salt("stream_msg_list")
        .auto_shrink(false)
        .show(ui, |ui| {
            if state.messages.is_empty() {
                ui.label(t("stream.no_messages"));
            } else {
                for (idx, msg) in state.messages.iter().enumerate() {
                    let seq = msg["sequence"].as_u64().unwrap_or(0);
                    let subject = msg["subject"].as_str().unwrap_or("");
                    let time_str = msg["time"]
                        .as_str()
                        .and_then(format_rfc3339_short)
                        .unwrap_or_default();
                    let label = if time_str.is_empty() {
                        format!("#{seq} {subject}")
                    } else {
                        format!("#{seq} {time_str} {subject}")
                    };
                    let selected = state.selected_msg == Some(idx);
                    if ui.selectable_label(selected, &label).clicked() {
                        state.selected_msg = Some(idx);
                    }
                }
            }
        });
}

/// Format an RFC3339 timestamp to a compact local-friendly display.
fn format_rfc3339_short(rfc: &str) -> Option<String> {
    // Show the date+time portion only (trim timezone/nanos for compact display)
    let trimmed = rfc.replace('T', " ");
    let trimmed = trimmed.trim_end_matches('Z');
    // Keep up to seconds precision
    Some(trimmed.split('.').next()?.to_string())
}

fn default_publish_subject(state: &StreamState) -> String {
    if let Some(idx) = state.selected_msg
        && let Some(subject) = state
            .messages
            .get(idx)
            .and_then(|msg| msg["subject"].as_str())
        && is_publishable_subject(subject)
    {
        return subject.to_string();
    }

    let filter = state.subject_filter.trim();
    if is_publishable_subject(filter) {
        return filter.to_string();
    }

    if let Some(info) = &state.info
        && let Some(subjects) = info["config"]["subjects"].as_array()
        && let Some(subject) = subjects
            .iter()
            .filter_map(|item| item.as_str())
            .find(|subject| is_publishable_subject(subject))
    {
        return subject.to_string();
    }

    String::new()
}

fn is_publishable_subject(subject: &str) -> bool {
    !subject.is_empty() && !subject.contains('*') && !subject.contains('>')
}

/// Convert a `SystemTime` to an RFC3339 UTC string.
fn system_time_to_rfc3339(time: SystemTime) -> String {
    let dur = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let (days, rem) = (secs / 86400, secs % 86400);
    let (hours, rem) = (rem / 3600, rem % 3600);
    let (mins, s) = (rem / 60, rem % 60);

    // Days since 1970-01-01 → y/m/d via a simple calendar walk
    let (y, m, d) = epoch_days_to_ymd(days);
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{mins:02}:{s:02}Z")
}

fn epoch_days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        y += 1;
    }
    let month_days: [u64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0;
    for md in &month_days {
        if days < *md {
            break;
        }
        days -= md;
        m += 1;
    }
    (y, m + 1, days + 1)
}

fn is_leap(y: u64) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

fn render_purge_controls(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    state: &mut StreamState,
    backend: &BackendHandle,
) {
    egui::CollapsingHeader::new(t("stream.purge"))
        .id_salt("stream_purge")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(t("stream.purge_subject"));
                ui.text_edit_singleline(&mut state.purge_subject);
            });
            ui.horizontal(|ui| {
                if ui.button(t("stream.purge_filtered")).clicked()
                    && !state.purge_subject.is_empty()
                {
                    backend.send(BackendCommand::PurgeStream {
                        connection_id,
                        name: stream_name.to_string(),
                        filter: Some(state.purge_subject.clone()),
                    });
                }
                if ui.button(t("stream.purge_all")).clicked() {
                    backend.send(BackendCommand::PurgeStream {
                        connection_id,
                        name: stream_name.to_string(),
                        filter: None,
                    });
                }
            });
            if let Some(idx) = state.selected_msg
                && let Some(msg) = state.messages.get(idx)
                && let Some(seq) = msg["sequence"].as_u64()
                && ui
                    .button(format!("{} #{seq}", t("stream.delete_msg")))
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
                    ui.label(t("stream.name"));
                    ui.label(name);
                    ui.end_row();
                }
                if let Some(subjects) = config.get("subjects").and_then(|v| v.as_array()) {
                    ui.label(t("stream.subjects"));
                    ui.label(
                        subjects
                            .iter()
                            .filter_map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    ui.end_row();
                }
                if let Some(storage) = config.get("storage").and_then(|v| v.as_str()) {
                    ui.label(t("stream.storage"));
                    ui.label(storage);
                    ui.end_row();
                }
                if let Some(retention) = config.get("retention").and_then(|v| v.as_str()) {
                    ui.label(t("stream.retention"));
                    ui.label(retention);
                    ui.end_row();
                }
            }
            if let Some(st) = info.get("state") {
                for (label, value, is_bytes) in [
                    (
                        t("stream.msg_count"),
                        st.get("messages").and_then(|v| v.as_u64()),
                        false,
                    ),
                    (
                        t("stream.bytes"),
                        st.get("bytes").and_then(|v| v.as_u64()),
                        true,
                    ),
                    (
                        t("stream.consumers"),
                        st.get("consumer_count").and_then(|v| v.as_u64()),
                        false,
                    ),
                ] {
                    if let Some(value) = value {
                        ui.label(label);
                        if is_bytes {
                            ui.label(format_bytes(value));
                        } else {
                            ui.label(value.to_string());
                        }
                        ui.end_row();
                    }
                }
            }
        });
}

fn stream_message_detail(
    ui: &mut egui::Ui,
    msg: &serde_json::Value,
    payload_format: &mut PayloadFormat,
    proto_view: &mut crate::proto::ProtoViewState,
    proto_manager: &ProtoSchemaManager,
) {
    ui.horizontal(|ui| {
        ui.label(t("stream.msg_detail"));
        format::format_selector(ui, "stream_msg_fmt", payload_format);
    });

    egui::Grid::new("stream_msg_detail_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            if let Some(seq) = msg["sequence"].as_u64() {
                ui.label(t("stream.msg_sequence"));
                ui.label(seq.to_string());
                ui.end_row();
            }
            if let Some(subject) = msg["subject"].as_str() {
                ui.label(t("stream.msg_subject"));
                ui.label(subject);
                ui.end_row();
            }
            if let Some(time) = msg["time"].as_str()
                && !time.is_empty()
            {
                ui.label(t("stream.msg_time"));
                ui.label(time);
                ui.end_row();
            }
        });

    if let Some(headers) = msg["headers"].as_array()
        && !headers.is_empty()
    {
        ui.add_space(4.0);
        ui.label(t("stream.msg_headers"));
        egui::Grid::new("stream_msg_headers")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for h in headers {
                    if let Some(arr) = h.as_array()
                        && arr.len() == 2
                    {
                        ui.label(arr[0].as_str().unwrap_or(""));
                        ui.label(arr[1].as_str().unwrap_or(""));
                        ui.end_row();
                    }
                }
            });
    }

    ui.add_space(4.0);
    ui.label(t("stream.msg_payload"));
    if let Some(payload_b64) = msg["payload_base64"].as_str() {
        match base64::engine::general_purpose::STANDARD.decode(payload_b64) {
            Ok(data) => format::render_payload_with_proto(
                ui,
                &data,
                *payload_format,
                "stream_proto",
                proto_view,
                proto_manager,
            ),
            Err(_) => {
                ui.label(t("stream.invalid_base64"));
            }
        }
    } else {
        ui.label(t("stream.no_payload"));
    }
}
