use base64::Engine;
use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

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
    render_message_controls(ui, connection_id, stream_name, state, backend);
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
        // Bottom: consumers + purge (resizable)
        egui::Panel::bottom(egui::Id::new((
            "stream_right_bottom",
            connection_id,
            stream_name,
        )))
        .resizable(true)
        .default_size(150.0)
        .show_inside(ui, |ui| {
            render_consumers(ui, connection_id, stream_name, state, backend, actions);
            ui.separator();
            render_purge_controls(ui, connection_id, stream_name, state, backend);
        });

        // Remaining space: message detail
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
}

fn render_message_controls(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    state: &mut StreamState,
    backend: &BackendHandle,
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
        if ui
            .add_enabled(!state.fetching, egui::Button::new(t("stream.fetch")))
            .clicked()
        {
            backend.send(BackendCommand::GetStreamMessages {
                connection_id,
                stream: stream_name.to_string(),
                start_sequence: state.start_seq.parse().ok(),
                subject_filter: (!state.subject_filter.is_empty())
                    .then(|| state.subject_filter.clone()),
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
                    let label = format!("#{seq} {subject}");
                    let selected = state.selected_msg == Some(idx);
                    if ui.selectable_label(selected, &label).clicked() {
                        state.selected_msg = Some(idx);
                    }
                }
            }
        });
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
    egui::ScrollArea::vertical()
        .id_salt("stream_msg_payload")
        .show(ui, |ui| {
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
        });
}
