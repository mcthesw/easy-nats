use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle, KvBucketInfo};

use crate::format;
use crate::i18n::t;
use crate::schema::{MessageSchemaManager, kv_subject};
use crate::tabs::guard::TabGuard;
use crate::theme::SyntaxPalette;

use super::common::{
    KV_VALUE_SEARCH_BATCH, SearchStatus, auto_refresh_ui, format_bytes,
    payload_input_format_selector, render_search_row,
};
use super::types::{KvBucketState, TabAction};

mod key_filter;

#[allow(clippy::too_many_arguments)]
pub fn kv_bucket_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
    schema_manager: &MessageSchemaManager,
    guard: &TabGuard,
    syntax_palette: SyntaxPalette,
) {
    ui.horizontal(|ui| {
        ui.heading(bucket_name);
        if let Some(info) = &state.info
            && ui.button(t("kv.edit_bucket")).clicked()
        {
            actions.push(TabAction::OpenKvBucketEdit {
                connection_id,
                bucket_info: info.clone(),
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
        let new_gen = crate::tabs::next_generation();
        state.load_generation = new_gen;
        state.keys.clear();
        state.fetched_values.clear();
        state.fetched_value_bytes.clear();
        key_filter::invalidate(state);
        state.value_search_cursor = 0;
        state.value_search_scanning = 0;
        state.value_search_pending.clear();
        state.search_generation = state.search_generation.wrapping_add(1);
        backend.send(BackendCommand::ListKvKeys {
            connection_id,
            bucket: bucket_name.to_string(),
            cancel: guard.cancellation(),
            generation: new_gen,
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
            .show(ui, |ui| kv_bucket_info_panel(ui, info, state));
    }
    ui.separator();

    // Horizontal split: left key list, right detail/history
    let panel_id = egui::Id::new(("kv_left_panel", connection_id, bucket_name));
    egui::Panel::left(panel_id)
        .resizable(true)
        .default_size(300.0)
        .size_range(200.0..=f32::INFINITY)
        .show(ui, |ui| {
            render_key_list(
                ui,
                connection_id,
                bucket_name,
                state,
                backend,
                actions,
                guard,
            );
        });

    // Right panel: detail or history
    egui::CentralPanel::default().show(ui, |ui| {
        if state.show_history {
            render_history(
                ui,
                connection_id,
                bucket_name,
                state,
                schema_manager,
                syntax_palette,
            );
        } else {
            render_detail_panel(
                ui,
                connection_id,
                bucket_name,
                state,
                backend,
                schema_manager,
                syntax_palette,
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
    guard: &TabGuard,
) {
    // Toolbar row: refresh + new entry
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!state.loading_entries, egui::Button::new("↻"))
            .on_hover_text(t("kv.refresh"))
            .clicked()
        {
            refresh_kv_keys(connection_id, bucket_name, state, backend, guard);
        }
        if ui.button("＋").on_hover_text(t("kv.new_entry")).clicked() {
            actions.push(TabAction::OpenKvEntryCreate {
                connection_id,
                bucket_name: bucket_name.to_string(),
                initial_key: state.search.query.trim().to_string(),
            });
        }
    });
    let search_changed = render_search_row(
        ui,
        ("kv_search", bucket_name),
        &mut state.search,
        t("kv.search_placeholder"),
        t("kv.search_scope_key"),
        t("kv.search_scope_value"),
    );
    if search_changed {
        state.search_more_requested = false;
        state.value_search_cursor = 0;
        key_filter::invalidate(state);
    }
    let search_status = kv_search_status(state);
    if state.search.is_active() {
        ui.horizontal_wrapped(|ui| {
            if let Some(text) = search_status.text() {
                ui.weak(text);
            }
            if state.search.secondary {
                ui.weak(format!(
                    "· {} {}/{}",
                    t("kv.search_values_scanned"),
                    state.fetched_values.len(),
                    state.keys.len()
                ));
            }
        });
    }
    render_kv_search_actions(ui, connection_id, bucket_name, state, backend, guard);

    if state.loading_entries {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(t("kv.loading_keys"));
        });
    }

    let row_count = key_filter::filtered_row_count(state);
    if row_count == 0 {
        ui.label(t("kv.no_keys"));
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt(("kv_keys", connection_id, bucket_name))
        .auto_shrink([false; 2])
        .show_rows(ui, 22.0, row_count, |ui, row_range| {
            let key_indices = key_filter::visible_key_indices(state, row_range);
            for key_index in key_indices {
                let Some(key) = state.keys.get(key_index).cloned() else {
                    continue;
                };
                let selected = state.selected_key.as_deref() == Some(key.as_str());
                let response = ui
                    .add(
                        egui::Button::selectable(selected, key.as_str())
                            .truncate()
                            .min_size(egui::vec2(ui.available_width(), 22.0)),
                    )
                    .on_hover_text(key.as_str());
                if response.clicked() {
                    state.selected_key = Some(key.clone());
                    key_filter::invalidate(state);
                    state.show_history = false;
                    // Clear editor state and request entry details
                    state.entry_key.clear();
                    state.entry_value.clear();
                    state.entry_revision = None;
                    state.entry_operation = None;
                    state.entry_created = None;
                    state.loading_entry = true;
                    backend.send(BackendCommand::GetKvEntry {
                        connection_id,
                        bucket: bucket_name.to_string(),
                        key: key.clone(),
                    });
                    backend.send(BackendCommand::GetKvHistory {
                        connection_id,
                        bucket: bucket_name.to_string(),
                        key: key.clone(),
                    });
                    state.loading_history = true;
                }
            }
        });
}

fn render_kv_search_actions(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
    guard: &TabGuard,
) {
    let value_search_active = state.search.is_active() && state.search.secondary;
    let can_scan_values = value_search_active
        && state.value_search_scanning == 0
        && state.value_search_cursor < state.keys.len();
    let can_load_keys = state.search.is_active() && !state.keys_complete && !state.loading_entries;

    if value_search_active || can_load_keys || state.value_search_scanning > 0 {
        ui.horizontal_wrapped(|ui| {
            if state.value_search_scanning > 0 {
                ui.spinner();
                ui.weak(t("kv.search_values_loading"));
            }
            if can_scan_values {
                let label = if state.value_search_cursor == 0 {
                    t("kv.search_values")
                } else {
                    t("kv.search_more_values")
                };
                if ui.small_button(label).clicked() {
                    scan_next_kv_values(connection_id, bucket_name, state, backend);
                }
            }
            if can_load_keys {
                ui.weak(t("kv.search_more_hint"));
                if ui.small_button(t("kv.search_more")).clicked() {
                    refresh_kv_keys(connection_id, bucket_name, state, backend, guard);
                }
            }
        });
    }
}

fn scan_next_kv_values(
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
) {
    let start = state.value_search_cursor.min(state.keys.len());
    let end = (start + KV_VALUE_SEARCH_BATCH).min(state.keys.len());
    let keys: Vec<String> = state.keys[start..end]
        .iter()
        .filter(|key| !state.fetched_values.contains_key(key.as_str()))
        .cloned()
        .collect();
    state.value_search_cursor = end;
    state.value_search_scanning += keys.len();
    for key in keys {
        state.value_search_pending.insert(key.clone());
        backend.send(BackendCommand::GetKvEntry {
            connection_id,
            bucket: bucket_name.to_string(),
            key,
        });
    }
}

fn kv_search_status(state: &mut KvBucketState) -> SearchStatus {
    if !state.search.is_active() {
        return SearchStatus::Inactive;
    }
    let matches = key_filter::filtered_row_count(state);
    SearchStatus::Showing {
        matches,
        capped: false,
    }
}

fn normalized_entry_key(key: &str) -> String {
    key.trim().to_owned()
}

fn refresh_kv_keys(
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
    guard: &TabGuard,
) {
    let new_gen = crate::tabs::next_generation();
    state.load_generation = new_gen;
    state.keys.clear();
    state.fetched_values.clear();
    state.fetched_value_bytes.clear();
    key_filter::invalidate(state);
    state.value_search_cursor = 0;
    state.value_search_scanning = 0;
    state.value_search_pending.clear();
    state.search_generation = state.search_generation.wrapping_add(1);
    state.keys_complete = false;
    state.loading_entries = true;
    backend.send(BackendCommand::ListKvKeys {
        connection_id,
        bucket: bucket_name.to_string(),
        cancel: guard.cancellation(),
        generation: new_gen,
    });
}

fn render_detail_panel(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    backend: &BackendHandle,
    schema_manager: &MessageSchemaManager,
    syntax_palette: SyntaxPalette,
) {
    let _form = crate::keyboard::Form::connected(ui, "render_detail_panel", false, connection_id);
    if state.selected_key.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label(t("kv.select_key_hint"));
        });
        return;
    }

    if state.loading_entry {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(t("kv.loading_entry"));
        });
        return;
    }

    // Metadata
    egui::Grid::new(("kv_detail_grid", connection_id, bucket_name))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("kv.key"));
            crate::keyboard::singleline(ui, &mut state.entry_key);
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
    ui.horizontal_wrapped(|ui| {
        ui.label(t("kv.value_editor"));
        ui.label(t("common.payload_input_format"));
        payload_input_format_selector(ui, "kv_value_input_fmt", &mut state.editor_input_format);
        ui.label(t("common.payload_preview_format"));
        format::format_selector(ui, "kv_value_fmt", &mut state.editor_format);
        if ui.small_button(t("kv.format_json")).clicked()
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&state.entry_value)
            && let Ok(pretty) = serde_json::to_string_pretty(&val)
        {
            state.entry_value = pretty;
        }
        render_generate_json_button(
            ui,
            &schema_manager.payload_template(
                connection_id,
                &kv_subject(bucket_name, &normalized_entry_key(&state.entry_key)),
            ),
            &mut state.entry_value,
        );
    });
    egui::ScrollArea::vertical()
        .id_salt(("kv_value_editor", connection_id, bucket_name))
        .max_height((ui.available_height() * 0.5).max(80.0))
        .show(ui, |ui| {
            crate::keyboard::text_edit(
                ui,
                egui::TextEdit::multiline(&mut state.entry_value)
                    .desired_width(f32::INFINITY)
                    .desired_rows(6)
                    .code_editor()
                    .lock_focus(false),
                true,
            );
        });

    ui.add_space(4.0);
    // Action toolbar
    let entry_key = normalized_entry_key(&state.entry_key);
    let can_save = !entry_key.is_empty();
    let subject = kv_subject(bucket_name, &entry_key);
    let outgoing_preview = if can_save {
        Some(schema_manager.prepare_outgoing_with_input_format(
            connection_id,
            &subject,
            &state.entry_value,
            state.editor_input_format,
        ))
    } else {
        None
    };
    ui.horizontal(|ui| {
        if crate::keyboard::primary_button(
            ui,
            can_save
                && outgoing_preview
                    .as_ref()
                    .is_none_or(|outgoing| outgoing.can_send),
            t("common.save"),
        ) {
            let payload = outgoing_preview
                .as_ref()
                .map(|outgoing| outgoing.payload.clone())
                .unwrap_or_else(|| state.entry_value.as_bytes().to_vec());
            backend.send(BackendCommand::PutKvEntry {
                connection_id,
                bucket: bucket_name.to_string(),
                key: entry_key.clone(),
                value: payload,
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
                key: entry_key.clone(),
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
                key: entry_key.clone(),
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
    if let Some(status) = outgoing_preview
        .as_ref()
        .and_then(|outgoing| outgoing.status.as_ref())
    {
        format::render_schema_status(ui, status);
    }

    ui.separator();

    ui.label(t("kv.value_preview"));
    let preview_bytes = outgoing_preview
        .as_ref()
        .filter(|outgoing| outgoing.can_send)
        .map(|outgoing| outgoing.payload.as_slice())
        .unwrap_or_else(|| state.entry_value.as_bytes());
    egui::ScrollArea::vertical()
        .id_salt(("kv_value_preview", connection_id, bucket_name))
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            format::render_payload_with_proto(
                ui,
                preview_bytes,
                state.editor_format,
                "kv_editor_proto",
                &mut state.editor_proto_view,
                schema_manager,
                syntax_palette,
            );
        });
}

fn render_history(
    ui: &mut egui::Ui,
    connection_id: u64,
    bucket_name: &str,
    state: &mut KvBucketState,
    schema_manager: &MessageSchemaManager,
    syntax_palette: SyntaxPalette,
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
            .max_height(history_scroll_height(ui.available_height()))
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for item in &state.history {
                    let revision = item.revision;
                    let operation = item.operation.as_str();
                    let created = item.created.as_str();
                    egui::CollapsingHeader::new(format!("r{revision} — {operation} — {created}"))
                        .id_salt(("kv_history_item", connection_id, bucket_name, revision))
                        .show(ui, |ui| {
                            let subject =
                                kv_subject(bucket_name, &normalized_entry_key(&state.entry_key));
                            format::render_payload_with_schema(
                                ui,
                                &item.value,
                                state.history_format,
                                "kv_history_proto",
                                &mut state.history_proto_view,
                                format::SchemaRenderContext {
                                    manager: schema_manager,
                                    connection_id,
                                    subject: &subject,
                                    syntax_palette,
                                },
                            );
                        });
                }
            });
    }
}

fn history_scroll_height(available_height: f32) -> f32 {
    available_height.max(80.0)
}

fn render_generate_json_button(
    ui: &mut egui::Ui,
    payload_template: &Result<Option<String>, String>,
    payload: &mut String,
) {
    let template = payload_template
        .as_ref()
        .ok()
        .and_then(|value| value.as_ref());
    let response = ui.add_enabled(
        template.is_some(),
        egui::Button::new(t("publisher.generate_json")),
    );
    if response.clicked()
        && let Some(template) = template
    {
        *payload = template.clone();
    }
    match payload_template {
        Ok(Some(_)) => {}
        Ok(None) => {
            response.on_hover_text(t("publisher.generate_json_unavailable"));
        }
        Err(error) => {
            response.on_hover_text(error);
        }
    }
}

fn current_kv_count_text(state: &KvBucketState) -> String {
    let suffix = if state.keys_complete { "" } else { "+" };
    format!("{}{}", state.keys.len(), suffix)
}

fn kv_bucket_info_panel(ui: &mut egui::Ui, info: &KvBucketInfo, state: &KvBucketState) {
    egui::Grid::new("kv_bucket_info_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in [
                (t("kv.bucket"), Some(info.bucket.clone())),
                (t("kv.current_keys"), Some(current_kv_count_text(state))),
                (
                    t("kv.values_stored"),
                    Some(info.stored_history_values.to_string()),
                ),
                (t("kv.history_depth"), Some(info.history_depth.to_string())),
                (t("kv.storage"), Some(info.storage.clone())),
                (t("kv.bytes"), Some(format_bytes(info.bytes))),
            ] {
                ui.label(label);
                ui.label(value.unwrap_or_default());
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_key_search_matches_loaded_keys() {
        let mut state = KvBucketState {
            keys: vec!["users.alice".to_string(), "orders.1".to_string()],
            ..Default::default()
        };
        state.search.query = "alice".to_string();
        state.search.primary = true;
        state.search.secondary = false;

        assert_eq!(key_filter::visible_key_indices(&mut state, 0..1), vec![0]);
    }

    #[test]
    fn kv_value_scan_advances_in_batches() {
        let mut state = KvBucketState {
            keys: (0..150).map(|idx| format!("key.{idx}")).collect(),
            ..Default::default()
        };
        state.value_search_cursor = 100;
        assert_eq!(state.value_search_cursor, 100);
        assert_eq!(state.keys.len(), 150);
    }

    #[test]
    fn current_kv_count_marks_incomplete_key_lists() {
        let mut state = KvBucketState {
            keys: vec!["a".to_string(), "b".to_string()],
            ..Default::default()
        };
        assert_eq!(current_kv_count_text(&state), "2+");

        state.keys_complete = true;
        assert_eq!(current_kv_count_text(&state), "2");
    }

    #[test]
    fn kv_value_search_uses_fetched_value_cache() {
        let mut state = KvBucketState {
            keys: vec!["users.alice".to_string(), "users.bob".to_string()],
            ..Default::default()
        };
        state
            .fetched_values
            .insert("users.bob".to_string(), "balance: 42".to_string());
        state.search.query = "42".to_string();
        state.search.primary = false;
        state.search.secondary = true;

        assert_eq!(key_filter::visible_key_indices(&mut state, 0..1), vec![1]);
    }

    #[test]
    fn normalized_entry_key_trims_schema_and_write_target() {
        assert_eq!(normalized_entry_key("  orders.1  "), "orders.1");
        assert_eq!(normalized_entry_key("\tusers.alice\n"), "users.alice");
    }

    #[test]
    fn history_scroll_height_uses_available_panel_space() {
        assert_eq!(history_scroll_height(640.0), 640.0);
        assert_eq!(history_scroll_height(12.0), 80.0);
    }
}
