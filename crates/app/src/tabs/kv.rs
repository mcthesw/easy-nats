use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format;
use crate::i18n::t;
use crate::proto::ProtoSchemaManager;

use super::common::{auto_refresh_ui, decode_base64_payload, format_bytes};
use super::types::{KvBucketState, TabAction};

pub fn kv_bucket_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
    proto_manager: &ProtoSchemaManager,
) {
    ui.horizontal(|ui| {
        ui.heading(bucket_name);
        if let Some(info) = &state.info
            && ui.button(t("kv.edit_bucket")).clicked()
        {
            actions.push(TabAction::OpenKvBucketEdit {
                connection_id,
                bucket_json: info.clone(),
            });
        }
        if ui.button(t("kv.delete_bucket")).clicked() {
            actions.push(TabAction::ConfirmDeleteKvBucket {
                connection_id,
                bucket_name: bucket_name.to_string(),
            });
        }
    });

    // Auto-refresh (outside the split to avoid panel width jitter)
    ui.horizontal(|ui| {
        auto_refresh_ui(ui, "kv_auto_refresh", &mut state.auto_refresh);
    });
    if state.auto_refresh.should_refresh() {
        backend.send(BackendCommand::ListKvKeys {
            connection_id,
            bucket: bucket_name.to_string(),
        });
        state.loading_entries = true;
        state.auto_refresh.mark_refreshed();
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
    } else if state.auto_refresh.enabled {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
    }

    if let Some(info) = &state.info {
        egui::CollapsingHeader::new(t("kv.bucket_info"))
            .id_salt(("kv_bucket_info", connection_id, bucket_name))
            .default_open(false)
            .show(ui, |ui| kv_bucket_info_panel(ui, info));
    }
    ui.separator();

    // Horizontal split: left key list, right detail/history
    let panel_id = egui::Id::new(("kv_left_panel", connection_id, bucket_name));
    egui::Panel::left(panel_id)
        .resizable(true)
        .default_size(300.0)
        .size_range(200.0..=f32::INFINITY)
        .show_inside(ui, |ui| {
            render_key_list(ui, connection_id, bucket_name, state, backend, actions);
        });

    // Right panel: detail or history
    egui::CentralPanel::default().show_inside(ui, |ui| {
        if state.show_history {
            render_history(ui, connection_id, bucket_name, state, proto_manager);
        } else {
            render_detail_panel(
                ui,
                connection_id,
                bucket_name,
                state,
                backend,
                proto_manager,
            );
        }
    });
}

fn render_key_list(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
) {
    // Toolbar row: refresh + new entry
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!state.loading_entries, egui::Button::new("↻"))
            .on_hover_text(t("kv.refresh"))
            .clicked()
        {
            backend.send(BackendCommand::ListKvKeys {
                connection_id,
                bucket: bucket_name.to_string(),
            });
            state.loading_entries = true;
        }
        if ui.button("+").on_hover_text(t("kv.new_entry")).clicked() {
            actions.push(TabAction::OpenKvEntryCreate {
                connection_id,
                bucket_name: bucket_name.to_string(),
                initial_key: state.key_filter.trim().to_string(),
            });
        }
    });
    // Filter row: full width
    ui.add(
        egui::TextEdit::singleline(&mut state.key_filter)
            .hint_text(t("kv.key_filter"))
            .desired_width(f32::INFINITY),
    );

    if state.loading_entries {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(t("kv.loading_keys"));
        });
    }

    egui::ScrollArea::vertical()
        .id_salt(("kv_keys", connection_id, bucket_name))
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
                ui.label(t("kv.no_keys"));
            } else {
                for entry in &filtered {
                    let key = entry["key"].as_str().unwrap_or("");
                    if ui
                        .selectable_label(state.selected_key.as_deref() == Some(key), key)
                        .clicked()
                    {
                        state.selected_key = Some(key.to_string());
                        state.show_history = false;
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

fn render_detail_panel(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
    proto_manager: &ProtoSchemaManager,
) {
    if state.selected_key.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label(t("kv.select_key_hint"));
        });
        return;
    }

    // Action toolbar
    ui.horizontal(|ui| {
        let can_save = !state.entry_key.trim().is_empty();
        if ui
            .add_enabled(can_save, egui::Button::new(t("common.save")))
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
            .add_enabled(can_save, egui::Button::new(t("kv.delete_entry")))
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
            .add_enabled(can_save, egui::Button::new(t("kv.purge_entry")))
            .clicked()
        {
            backend.send(BackendCommand::PurgeKvEntry {
                connection_id,
                bucket: bucket_name.to_string(),
                key: state.entry_key.clone(),
            });
            state.loading_entries = true;
        }
        if ui
            .add_enabled(can_save, egui::Button::new(t("kv.history")))
            .clicked()
        {
            state.show_history = true;
        }
    });

    ui.separator();

    // Metadata
    egui::Grid::new(("kv_detail_grid", connection_id, bucket_name))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("kv.key"));
            ui.text_edit_singleline(&mut state.entry_key);
            ui.end_row();

            ui.label(t("kv.revision"));
            ui.label(
                state
                    .entry_revision
                    .map(|rev| rev.to_string())
                    .unwrap_or_else(|| t("kv.none").to_string()),
            );
            ui.end_row();

            ui.label(t("kv.operation"));
            ui.label(state.entry_operation.as_deref().unwrap_or(t("kv.none")));
            ui.end_row();

            ui.label(t("kv.created"));
            ui.label(state.entry_created.as_deref().unwrap_or(t("kv.none")));
            ui.end_row();
        });

    ui.separator();

    // Value editor
    ui.horizontal(|ui| {
        ui.label(t("kv.value_editor"));
        format::format_selector(ui, "kv_value_fmt", &mut state.editor_format);
        if ui.small_button(t("kv.format_json")).clicked()
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&state.entry_value)
            && let Ok(pretty) = serde_json::to_string_pretty(&val)
        {
            state.entry_value = pretty;
        }
    });
    egui::ScrollArea::vertical()
        .id_salt(("kv_value_editor", connection_id, bucket_name))
        .max_height((ui.available_height() * 0.5).max(80.0))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.entry_value)
                    .desired_width(f32::INFINITY)
                    .desired_rows(6)
                    .code_editor(),
            );
        });

    ui.add_space(4.0);
    ui.label(t("kv.value_preview"));
    egui::ScrollArea::vertical()
        .id_salt(("kv_value_preview", connection_id, bucket_name))
        .show(ui, |ui| {
            format::render_payload_with_proto(
                ui,
                state.entry_value.as_bytes(),
                state.editor_format,
                "kv_editor_proto",
                &mut state.editor_proto_view,
                proto_manager,
            );
        });
}

fn render_history(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    proto_manager: &ProtoSchemaManager,
) {
    ui.horizontal(|ui| {
        if ui.button(t("kv.back_to_detail")).clicked() {
            state.show_history = false;
        }
        ui.label(t("kv.history"));
        format::format_selector(ui, "kv_history_fmt", &mut state.history_format);
    });
    ui.separator();

    if state.loading_history {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(t("kv.loading_history"));
        });
    } else if state.history.is_empty() {
        ui.label(t("kv.no_history"));
    } else {
        egui::ScrollArea::vertical()
            .id_salt(("kv_history", connection_id, bucket_name))
            .max_height(220.0)
            .show(ui, |ui| {
                for item in &state.history {
                    let revision = item["revision"].as_u64().unwrap_or(0);
                    let operation = item["operation"].as_str().unwrap_or(t("kv.none"));
                    let created = item["created"].as_str().unwrap_or("");
                    egui::CollapsingHeader::new(format!("r{revision} — {operation} — {created}"))
                        .id_salt(("kv_history_item", connection_id, bucket_name, revision))
                        .show(ui, |ui| {
                            let payload = decode_base64_payload(item["value_base64"].as_str());
                            format::render_payload_with_proto(
                                ui,
                                &payload,
                                state.history_format,
                                "kv_history_proto",
                                &mut state.history_proto_view,
                                proto_manager,
                            );
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
                (t("kv.bucket"), info["bucket"].as_str().map(str::to_owned)),
                (
                    t("kv.values"),
                    Some(info["values"].as_u64().unwrap_or(0).to_string()),
                ),
                (
                    t("kv.history_depth"),
                    Some(info["history"].as_i64().unwrap_or(0).to_string()),
                ),
                (t("kv.storage"), info["storage"].as_str().map(str::to_owned)),
                (
                    t("kv.bytes"),
                    Some(format_bytes(info["bytes"].as_u64().unwrap_or(0))),
                ),
            ] {
                ui.label(label);
                ui.label(value.unwrap_or_default());
                ui.end_row();
            }
        });
}

fn load_kv_entry_into_editor(state: &mut KvBucketState, entry: &serde_json::Value) {
    state.entry_key = entry["key"].as_str().unwrap_or("").to_string();
    state.entry_value =
        String::from_utf8_lossy(&decode_base64_payload(entry["value_base64"].as_str())).to_string();
    state.entry_revision = entry["revision"].as_u64();
    state.entry_operation = entry["operation"].as_str().map(str::to_owned);
    state.entry_created = entry["created"].as_str().map(str::to_owned);
}
