use eframe::egui;

use crate::i18n::t;
use crate::schema::{
    MessageSchemaManager, SchemaSelector, SchemaSourceKind, SchemaSourceState, SubjectPattern,
    ValidationPolicy,
};

use super::types::{MessageSchemasState, TabAction};

pub fn message_schemas_ui(
    ui: &mut egui::Ui,
    state: &mut MessageSchemasState,
    manager: &MessageSchemaManager,
    connections: &[(u64, String)],
    actions: &mut Vec<TabAction>,
) {
    ui.heading(t("message_schema.title"));
    ui.separator();

    render_source_editor(ui, state, actions);
    ui.add_space(10.0);
    render_sources(ui, manager, actions);
    ui.add_space(14.0);
    ui.separator();
    ui.add_space(10.0);
    render_binding_editor(ui, state, manager, connections, actions);
    ui.add_space(10.0);
    render_bindings(ui, manager, connections, actions);
}

fn render_source_editor(
    ui: &mut egui::Ui,
    state: &mut MessageSchemasState,
    actions: &mut Vec<TabAction>,
) {
    ui.label(egui::RichText::new(t("message_schema.sources")).strong());
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.source_name)
                .hint_text(t("message_schema.source_name"))
                .desired_width(150.0),
        );
        egui::ComboBox::from_id_salt("schema_source_kind")
            .selected_text(t(state.source_kind.label_key()))
            .show_ui(ui, |ui| {
                for kind in SchemaSourceKind::ALL {
                    ui.selectable_value(&mut state.source_kind, kind, t(kind.label_key()));
                }
            });
        ui.add(
            egui::TextEdit::singleline(&mut state.source_path)
                .hint_text(t("message_schema.source_path"))
                .desired_width(280.0),
        );
        if ui.button(t("message_schema.browse")).clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let picked = match state.source_kind {
                    SchemaSourceKind::Protobuf => rfd::FileDialog::new().pick_folder(),
                    SchemaSourceKind::JsonSchema => rfd::FileDialog::new()
                        .add_filter("JSON Schema", &["json"])
                        .pick_file(),
                };
                if let Some(path) = picked {
                    state.source_path = path.to_string_lossy().to_string();
                }
            }
        }
        let can_add = !state.source_path.trim().is_empty();
        if ui
            .add_enabled(can_add, egui::Button::new(t("message_schema.add_source")))
            .clicked()
        {
            let name = if state.source_name.trim().is_empty() {
                fallback_name(&state.source_path)
            } else {
                state.source_name.trim().to_string()
            };
            actions.push(TabAction::AddMessageSchemaSource {
                name,
                kind: state.source_kind,
                path: state.source_path.trim().to_string(),
            });
            state.source_name.clear();
            state.source_path.clear();
        }
    });
}

fn render_sources(ui: &mut egui::Ui, manager: &MessageSchemaManager, actions: &mut Vec<TabAction>) {
    if manager.config().sources.is_empty() {
        ui.weak(t("message_schema.no_sources"));
        return;
    }

    egui::ScrollArea::horizontal()
        .id_salt("message_schema_sources_scroll")
        .show(ui, |ui| {
            egui::Grid::new("message_schema_sources")
                .num_columns(7)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(t("message_schema.enabled"));
                    ui.strong(t("message_schema.actions"));
                    ui.strong(t("message_schema.name"));
                    ui.strong(t("message_schema.kind"));
                    ui.strong(t("message_schema.path"));
                    ui.strong(t("message_schema.status"));
                    ui.strong(t("message_schema.entries"));
                    ui.end_row();

                    for source in &manager.config().sources {
                        let mut enabled = source.enabled;
                        if ui.checkbox(&mut enabled, "").changed() {
                            actions.push(TabAction::SetMessageSchemaSourceEnabled {
                                source_id: source.id,
                                enabled,
                            });
                        }
                        ui.horizontal(|ui| {
                            if ui.small_button(t("message_schema.reload")).clicked() {
                                actions.push(TabAction::ReloadMessageSchemaSource {
                                    source_id: source.id,
                                });
                            }
                            if ui.small_button(t("message_schema.remove")).clicked() {
                                actions.push(TabAction::RemoveMessageSchemaSource {
                                    source_id: source.id,
                                });
                            }
                        });
                        clipped_label(ui, &source.name, 170.0);
                        clipped_label(ui, t(source.kind.label_key()), 110.0);
                        clipped_monospace(ui, &source.path, 420.0);
                        let status = manager.status(source.id);
                        clipped_label(ui, &source_status_text(status), 120.0);
                        ui.label(
                            status
                                .map(|status| status.entries.len().to_string())
                                .unwrap_or_else(|| "0".to_string()),
                        );
                        ui.end_row();
                    }
                });
        });
}

fn render_binding_editor(
    ui: &mut egui::Ui,
    state: &mut MessageSchemasState,
    manager: &MessageSchemaManager,
    connections: &[(u64, String)],
    actions: &mut Vec<TabAction>,
) {
    ui.label(egui::RichText::new(t("message_schema.bindings")).strong());
    if manager.config().sources.is_empty() {
        ui.weak(t("message_schema.add_source_first"));
        return;
    }

    if state.binding_source_id.is_none() {
        state.binding_source_id = manager.config().sources.first().map(|source| source.id);
    }

    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.binding_name)
                .hint_text(t("message_schema.binding_name"))
                .desired_width(140.0),
        );
        egui::ComboBox::from_id_salt("schema_binding_connection")
            .width(155.0)
            .selected_text(connection_label(state.binding_connection_id, connections))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut state.binding_connection_id,
                    None,
                    t("message_schema.all_connections"),
                );
                for (id, name) in connections {
                    ui.selectable_value(
                        &mut state.binding_connection_id,
                        Some(*id),
                        format!("{name} #{id}"),
                    );
                }
            });
        ui.add(
            egui::TextEdit::singleline(&mut state.binding_subject_pattern)
                .hint_text(t("message_schema.subject_pattern"))
                .desired_width(170.0),
        );
        render_source_selector(ui, state, manager);
        render_entry_selector(ui, state, manager);
        egui::ComboBox::from_id_salt("schema_binding_policy")
            .width(95.0)
            .selected_text(t(state.binding_policy.label_key()))
            .show_ui(ui, |ui| {
                for policy in ValidationPolicy::ALL {
                    ui.selectable_value(&mut state.binding_policy, policy, t(policy.label_key()));
                }
            });
        if ui.button(t("message_schema.add_binding")).clicked() {
            match build_binding_selector(state, manager) {
                Ok((source_id, selector)) => {
                    if let Err(error) = SubjectPattern::parse(&state.binding_subject_pattern) {
                        state.last_error = Some(error);
                    } else {
                        let name = if state.binding_name.trim().is_empty() {
                            state.binding_subject_pattern.trim().to_string()
                        } else {
                            state.binding_name.trim().to_string()
                        };
                        actions.push(TabAction::AddMessageSchemaBinding {
                            name,
                            connection_id: state.binding_connection_id,
                            subject_pattern: state.binding_subject_pattern.trim().to_string(),
                            source_id,
                            selector,
                            policy: state.binding_policy,
                        });
                        state.binding_name.clear();
                        state.binding_subject_pattern.clear();
                        state.binding_schema_entry.clear();
                        state.last_error = None;
                    }
                }
                Err(error) => state.last_error = Some(error),
            }
        }
    });

    if let Some(error) = &state.last_error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
}

fn render_source_selector(
    ui: &mut egui::Ui,
    state: &mut MessageSchemasState,
    manager: &MessageSchemaManager,
) {
    let selected_text = state
        .binding_source_id
        .and_then(|id| {
            manager
                .config()
                .sources
                .iter()
                .find(|source| source.id == id)
        })
        .map(|source| source.name.clone())
        .unwrap_or_else(|| t("message_schema.select_source").to_string());
    egui::ComboBox::from_id_salt("schema_binding_source")
        .width(170.0)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for source in &manager.config().sources {
                if ui
                    .selectable_value(&mut state.binding_source_id, Some(source.id), &source.name)
                    .changed()
                {
                    state.binding_schema_entry.clear();
                }
            }
        });
}

fn render_entry_selector(
    ui: &mut egui::Ui,
    state: &mut MessageSchemasState,
    manager: &MessageSchemaManager,
) {
    let entries = state
        .binding_source_id
        .map(|source_id| manager.source_entries(source_id))
        .unwrap_or(&[]);
    if state.binding_schema_entry.is_empty()
        && let Some(first) = entries.first()
    {
        state.binding_schema_entry = first.clone();
    }
    egui::ComboBox::from_id_salt("schema_binding_entry")
        .width(230.0)
        .selected_text(if state.binding_schema_entry.is_empty() {
            t("message_schema.select_schema").to_string()
        } else {
            state.binding_schema_entry.clone()
        })
        .show_ui(ui, |ui| {
            for entry in entries {
                ui.selectable_value(&mut state.binding_schema_entry, entry.clone(), entry);
            }
        });
}

fn render_bindings(
    ui: &mut egui::Ui,
    manager: &MessageSchemaManager,
    connections: &[(u64, String)],
    actions: &mut Vec<TabAction>,
) {
    if manager.config().bindings.is_empty() {
        ui.weak(t("message_schema.no_bindings"));
        return;
    }

    egui::ScrollArea::horizontal()
        .id_salt("message_schema_bindings_scroll")
        .show(ui, |ui| {
            egui::Grid::new("message_schema_bindings")
                .num_columns(8)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(t("message_schema.enabled"));
                    ui.strong(t("message_schema.actions"));
                    ui.strong(t("message_schema.name"));
                    ui.strong(t("message_schema.connection"));
                    ui.strong(t("message_schema.subject_pattern"));
                    ui.strong(t("message_schema.schema"));
                    ui.strong(t("message_schema.policy"));
                    ui.strong(t("message_schema.status"));
                    ui.end_row();

                    for binding in &manager.config().bindings {
                        let mut enabled = binding.enabled;
                        if ui.checkbox(&mut enabled, "").changed() {
                            actions.push(TabAction::SetMessageSchemaBindingEnabled {
                                binding_id: binding.id,
                                enabled,
                            });
                        }
                        if ui.small_button(t("message_schema.remove")).clicked() {
                            actions.push(TabAction::RemoveMessageSchemaBinding {
                                binding_id: binding.id,
                            });
                        }
                        clipped_label(ui, &binding.name, 150.0);
                        clipped_label(
                            ui,
                            &connection_label(binding.connection_id, connections),
                            170.0,
                        );
                        clipped_monospace(ui, &binding.subject_pattern, 220.0);
                        clipped_label(ui, binding.selector.entry(), 240.0);
                        clipped_label(ui, t(binding.policy.label_key()), 90.0);
                        clipped_label(
                            ui,
                            &binding_status_text(
                                manager,
                                binding.source_id,
                                binding.selector.entry(),
                            ),
                            110.0,
                        );
                        ui.end_row();
                    }
                });
        });
}

fn build_binding_selector(
    state: &MessageSchemasState,
    manager: &MessageSchemaManager,
) -> Result<(u64, SchemaSelector), String> {
    let source_id = state
        .binding_source_id
        .ok_or_else(|| t("message_schema.select_source").to_string())?;
    let source = manager
        .config()
        .sources
        .iter()
        .find(|source| source.id == source_id)
        .ok_or_else(|| t("message_schema.select_source").to_string())?;
    if state.binding_schema_entry.trim().is_empty() {
        return Err(t("message_schema.select_schema").to_string());
    }
    let selector = match source.kind {
        SchemaSourceKind::Protobuf => SchemaSelector::ProtobufMessage {
            type_name: state.binding_schema_entry.trim().to_string(),
        },
        SchemaSourceKind::JsonSchema => SchemaSelector::JsonSchema {
            entry: state.binding_schema_entry.trim().to_string(),
        },
    };
    Ok((source_id, selector))
}

fn source_status_text(status: Option<&crate::schema::SchemaSourceStatus>) -> String {
    match status {
        Some(status) => match status.state {
            SchemaSourceState::Disabled => t("message_schema.status_disabled").to_string(),
            SchemaSourceState::Loaded => t("message_schema.status_loaded").to_string(),
            SchemaSourceState::Error => status
                .message
                .clone()
                .unwrap_or_else(|| t("message_schema.status_error").to_string()),
        },
        None => t("message_schema.status_unknown").to_string(),
    }
}

fn binding_status_text(
    manager: &MessageSchemaManager,
    source_id: u64,
    schema_entry: &str,
) -> String {
    let Some(status) = manager.status(source_id) else {
        return t("message_schema.status_unknown").to_string();
    };
    match status.state {
        SchemaSourceState::Disabled => t("message_schema.status_disabled").to_string(),
        SchemaSourceState::Error => t("message_schema.status_error").to_string(),
        SchemaSourceState::Loaded if status.entries.iter().any(|entry| entry == schema_entry) => {
            t("message_schema.status_ready").to_string()
        }
        SchemaSourceState::Loaded => t("message_schema.status_missing_schema").to_string(),
    }
}

fn connection_label(connection_id: Option<u64>, connections: &[(u64, String)]) -> String {
    match connection_id {
        Some(id) => connections
            .iter()
            .find(|(connection_id, _)| *connection_id == id)
            .map(|(_, name)| format!("{name} #{id}"))
            .unwrap_or_else(|| format!("#{id}")),
        None => t("message_schema.all_connections").to_string(),
    }
}

fn fallback_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn clipped_label(ui: &mut egui::Ui, text: &str, width: f32) {
    ui.add_sized(
        [width, ui.spacing().interact_size.y],
        egui::Label::new(text).truncate(),
    )
    .on_hover_text(text);
}

fn clipped_monospace(ui: &mut egui::Ui, text: &str, width: f32) {
    ui.add_sized(
        [width, ui.spacing().interact_size.y],
        egui::Label::new(egui::RichText::new(text).monospace()).truncate(),
    )
    .on_hover_text(text);
}
