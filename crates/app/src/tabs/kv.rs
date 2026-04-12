use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format;
use crate::ui_strings as S;

use super::common::{decode_base64_payload, format_bytes, kv_empty_preview};
use super::types::{KvBucketState, TabAction};

pub fn kv_bucket_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
) {
    ui.horizontal(|ui| {
        ui.heading(bucket_name);
        if ui.button(S::KV_DELETE_BUCKET).clicked() {
            actions.push(TabAction::ConfirmDeleteKvBucket {
                connection_id,
                bucket_name: bucket_name.to_string(),
            });
        }
    });

    if let Some(info) = &state.info {
        egui::CollapsingHeader::new(S::KV_BUCKET_INFO)
            .id_salt(("kv_bucket_info", connection_id, bucket_name))
            .default_open(true)
            .show(ui, |ui| kv_bucket_info_panel(ui, info));
        ui.separator();
    }

    render_key_list(ui, connection_id, bucket_name, state, backend);
    ui.separator();
    render_value_editor(ui, connection_id, bucket_name, state, backend);
    ui.separator();
    render_history(ui, connection_id, bucket_name, state);
}

fn render_key_list(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
) {
    ui.horizontal(|ui| {
        ui.label(S::KV_KEY_FILTER);
        ui.text_edit_singleline(&mut state.key_filter);
        if ui
            .add_enabled(!state.loading_entries, egui::Button::new(S::KV_REFRESH))
            .clicked()
        {
            backend.send(BackendCommand::ListKvKeys {
                connection_id,
                bucket: bucket_name.to_string(),
            });
            state.loading_entries = true;
        }
        if ui.button(S::KV_NEW_ENTRY).clicked() {
            clear_kv_editor(state);
        }
    });

    if state.loading_entries {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(S::KV_LOADING_KEYS);
        });
    }

    ui.add_space(4.0);
    ui.label(S::KV_KEYS);
    egui::ScrollArea::vertical()
        .id_salt(("kv_keys", connection_id, bucket_name))
        .max_height(180.0)
        .show(ui, |ui| {
            let filtered: Vec<serde_json::Value> = state
                .entries
                .iter()
                .filter(|entry| {
                    let key = entry["key"].as_str().unwrap_or("");
                    state.key_filter.is_empty() || key.starts_with(&state.key_filter)
                })
                .cloned()
                .collect();

            if filtered.is_empty() {
                ui.label(S::KV_NO_KEYS);
            } else {
                for entry in &filtered {
                    let key = entry["key"].as_str().unwrap_or("");
                    let revision = entry["revision"].as_u64().unwrap_or(0);
                    let preview = kv_payload_preview(entry, 48);
                    let label = format!("{key} (r{revision}) — {preview}");
                    if ui
                        .selectable_label(state.selected_key.as_deref() == Some(key), label)
                        .clicked()
                    {
                        state.selected_key = Some(key.to_string());
                        load_kv_entry_into_editor(state, entry);
                        backend.send(BackendCommand::GetKvEntry {
                            connection_id,
                            bucket: bucket_name.to_string(),
                            key: key.to_string(),
                        });
                        backend.send(BackendCommand::GetKvHistory {
                            connection_id,
                            bucket: bucket_name.to_string(),
                            key: key.to_string(),
                        });
                        state.loading_history = true;
                    }
                }
            }
        });
}

fn render_value_editor(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
) {
    ui.horizontal(|ui| {
        let can_save = !state.entry_key.trim().is_empty();
        if ui
            .add_enabled(can_save, egui::Button::new(S::SAVE))
            .clicked()
        {
            backend.send(BackendCommand::PutKvEntry {
                connection_id,
                bucket: bucket_name.to_string(),
                key: state.entry_key.clone(),
                value: state.entry_value.as_bytes().to_vec(),
            });
            state.loading_entries = true;
        }
        if ui
            .add_enabled(can_save, egui::Button::new(S::KV_DELETE_ENTRY))
            .clicked()
        {
            backend.send(BackendCommand::DeleteKvEntry {
                connection_id,
                bucket: bucket_name.to_string(),
                key: state.entry_key.clone(),
            });
            state.loading_entries = true;
        }
        if ui
            .add_enabled(can_save, egui::Button::new(S::KV_PURGE_ENTRY))
            .clicked()
        {
            backend.send(BackendCommand::PurgeKvEntry {
                connection_id,
                bucket: bucket_name.to_string(),
                key: state.entry_key.clone(),
            });
            state.loading_entries = true;
        }
    });

    egui::Grid::new(("kv_detail_grid", connection_id, bucket_name))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(S::KV_KEY);
            ui.text_edit_singleline(&mut state.entry_key);
            ui.end_row();

            ui.label(S::KV_REVISION);
            ui.label(
                state
                    .entry_revision
                    .map(|rev| rev.to_string())
                    .unwrap_or_else(|| S::KV_NONE.to_string()),
            );
            ui.end_row();

            ui.label(S::KV_OPERATION);
            ui.label(state.entry_operation.as_deref().unwrap_or(S::KV_NONE));
            ui.end_row();

            ui.label(S::KV_CREATED);
            ui.label(state.entry_created.as_deref().unwrap_or(S::KV_NONE));
            ui.end_row();
        });

    ui.add_space(4.0);
    ui.label(S::KV_VALUE_EDITOR);
    egui::ScrollArea::vertical()
        .id_salt(("kv_value_editor", connection_id, bucket_name))
        .max_height(120.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.entry_value)
                    .desired_width(f32::INFINITY)
                    .desired_rows(6)
                    .code_editor(),
            );
        });

    ui.horizontal(|ui| {
        ui.label(S::KV_VALUE_PREVIEW);
        format::format_selector(ui, "kv_value_fmt", &mut state.editor_format);
    });
    egui::ScrollArea::vertical()
        .id_salt(("kv_value_preview", connection_id, bucket_name))
        .max_height(160.0)
        .show(ui, |ui| {
            format::render_payload(ui, state.entry_value.as_bytes(), state.editor_format);
        });
}

fn render_history(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
) {
    ui.horizontal(|ui| {
        ui.label(S::KV_HISTORY);
        format::format_selector(ui, "kv_history_fmt", &mut state.history_format);
    });

    if state.loading_history {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(S::KV_LOADING_HISTORY);
        });
    } else if state.history.is_empty() {
        ui.label(S::KV_NO_HISTORY);
    } else {
        egui::ScrollArea::vertical()
            .id_salt(("kv_history", connection_id, bucket_name))
            .max_height(220.0)
            .show(ui, |ui| {
                for item in &state.history {
                    let revision = item["revision"].as_u64().unwrap_or(0);
                    let operation = item["operation"].as_str().unwrap_or(S::KV_NONE);
                    let created = item["created"].as_str().unwrap_or("");
                    egui::CollapsingHeader::new(format!("r{revision} — {operation} — {created}"))
                        .id_salt(("kv_history_item", connection_id, bucket_name, revision))
                        .show(ui, |ui| {
                            let payload = decode_base64_payload(item["value_base64"].as_str());
                            format::render_payload(ui, &payload, state.history_format);
                        });
                }
            });
    }
}

fn kv_bucket_info_panel(ui: &mut egui::Ui, info: &serde_json::Value) {
    egui::Grid::new("kv_bucket_info_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in [
                (S::KV_BUCKET, info["bucket"].as_str().map(str::to_owned)),
                (
                    S::KV_VALUES,
                    Some(info["values"].as_u64().unwrap_or(0).to_string()),
                ),
                (
                    S::KV_HISTORY_DEPTH,
                    Some(info["history"].as_i64().unwrap_or(0).to_string()),
                ),
                (S::KV_STORAGE, info["storage"].as_str().map(str::to_owned)),
                (
                    S::KV_BYTES,
                    Some(format_bytes(info["bytes"].as_u64().unwrap_or(0))),
                ),
            ] {
                ui.label(label);
                ui.label(value.unwrap_or_default());
                ui.end_row();
            }
        });
}

fn clear_kv_editor(state: &mut KvBucketState) {
    state.selected_key = None;
    state.entry_key.clear();
    state.entry_value.clear();
    state.entry_revision = None;
    state.entry_operation = None;
    state.entry_created = None;
    state.history.clear();
    state.loading_history = false;
}

fn load_kv_entry_into_editor(state: &mut KvBucketState, entry: &serde_json::Value) {
    state.entry_key = entry["key"].as_str().unwrap_or("").to_string();
    state.entry_value =
        String::from_utf8_lossy(&decode_base64_payload(entry["value_base64"].as_str())).to_string();
    state.entry_revision = entry["revision"].as_u64();
    state.entry_operation = entry["operation"].as_str().map(str::to_owned);
    state.entry_created = entry["created"].as_str().map(str::to_owned);
}

fn kv_payload_preview(entry: &serde_json::Value, max_len: usize) -> String {
    let payload = decode_base64_payload(entry["value_base64"].as_str());
    kv_empty_preview(&payload, max_len)
}
