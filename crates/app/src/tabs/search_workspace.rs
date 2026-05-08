use eframe::egui;

use crate::i18n::t;

use super::common::{
    SEARCH_RESULT_LIMIT, format_timestamp, matches_query, searchable_payload_text,
};
use super::types::{
    SearchField, SearchRecordSnapshot, SearchResultIdentity, SearchResultLocator,
    SearchSourceCoverage, SearchSourceId, SearchSourceSnapshot, SearchWorkspaceCacheKey,
    SearchWorkspaceResult, SearchWorkspaceState, TabAction, TabKind,
};

const SOURCE_CHIP_LABEL_WIDTH: f32 = 190.0;
const SOURCE_CHIP_COVERAGE_WIDTH: f32 = 105.0;
const SOURCE_CHIP_FALLBACK_WIDTH: f32 = 125.0;

pub(crate) fn source_snapshot_from_tab(tab: &TabKind) -> Option<SearchSourceSnapshot> {
    match tab {
        TabKind::KvBucket {
            connection_id,
            connection_name,
            bucket_name,
            state,
            ..
        } => {
            let id = SearchSourceId::Kv {
                connection_id: *connection_id,
                bucket_name: bucket_name.clone(),
            };
            let mut records = Vec::new();
            for key in &state.keys {
                records.push(SearchRecordSnapshot {
                    field: SearchField::Key,
                    item_id: key.clone(),
                    item_label: key.clone(),
                    text: key.clone(),
                    snippet: compact_text(key, 160),
                    locator: SearchResultLocator::KvKey {
                        connection_id: *connection_id,
                        bucket_name: bucket_name.clone(),
                        key: key.clone(),
                    },
                });
            }
            let mut fetched_keys = state.fetched_values.keys().cloned().collect::<Vec<_>>();
            fetched_keys.sort();
            for key in fetched_keys {
                if let Some(value) = state.fetched_values.get(&key) {
                    records.push(SearchRecordSnapshot {
                        field: SearchField::Value,
                        item_id: key.clone(),
                        item_label: key.clone(),
                        text: value.clone(),
                        snippet: compact_text(value, 160),
                        locator: SearchResultLocator::KvKey {
                            connection_id: *connection_id,
                            bucket_name: bucket_name.clone(),
                            key,
                        },
                    });
                }
            }

            Some(SearchSourceSnapshot {
                id,
                label: format!("{bucket_name} ({connection_name})"),
                generation: state.search_generation,
                coverage: SearchSourceCoverage::Kv {
                    loaded_keys: state.keys.len(),
                    fetched_values: state.fetched_values.len(),
                    scanning: state.value_search_scanning,
                    can_scan_more: state.value_search_scanning == 0
                        && state.value_search_cursor < state.keys.len(),
                },
                records,
            })
        }
        TabKind::Stream {
            connection_id,
            connection_name,
            stream_name,
            state,
            ..
        } => {
            let id = SearchSourceId::Stream {
                connection_id: *connection_id,
                stream_name: stream_name.clone(),
            };
            let mut records = Vec::new();
            for msg in &state.messages {
                let sequence = msg.sequence;
                let subject = msg.subject.as_str();
                let item_label = if subject.is_empty() {
                    format!("#{sequence}")
                } else {
                    format!("#{sequence} {subject}")
                };
                if !subject.is_empty() {
                    records.push(SearchRecordSnapshot {
                        field: SearchField::Subject,
                        item_id: sequence.to_string(),
                        item_label: item_label.clone(),
                        text: subject.to_string(),
                        snippet: compact_text(subject, 160),
                        locator: SearchResultLocator::StreamMessage {
                            connection_id: *connection_id,
                            stream_name: stream_name.clone(),
                            sequence,
                        },
                    });
                }
                let payload = searchable_payload_text(&msg.payload);
                if !payload.is_empty() {
                    records.push(SearchRecordSnapshot {
                        field: SearchField::Payload,
                        item_id: sequence.to_string(),
                        item_label: item_label.clone(),
                        text: payload.clone(),
                        snippet: compact_text(&payload, 160),
                        locator: SearchResultLocator::StreamMessage {
                            connection_id: *connection_id,
                            stream_name: stream_name.clone(),
                            sequence,
                        },
                    });
                }
            }

            Some(SearchSourceSnapshot {
                id,
                label: format!("{stream_name} ({connection_name})"),
                generation: state.search_generation,
                coverage: SearchSourceCoverage::Stream {
                    messages: state.messages.len(),
                },
                records,
            })
        }
        TabKind::Subscriber {
            connection_id,
            connection_name,
            backend_id,
            guard,
            state,
            ..
        } => {
            let id = SearchSourceId::Subscriber {
                connection_id: *connection_id,
                backend_id: *backend_id,
            };
            let display_id = guard
                .display_id()
                .map(|id| format!("#{id} "))
                .unwrap_or_default();
            let mut records = Vec::new();
            for msg in &state.messages {
                let item_label = format!("{} {}", format_timestamp(msg.timestamp), msg.subject);
                records.push(SearchRecordSnapshot {
                    field: SearchField::Subject,
                    item_id: msg.id.to_string(),
                    item_label: item_label.clone(),
                    text: msg.subject.clone(),
                    snippet: compact_text(&msg.subject, 160),
                    locator: SearchResultLocator::SubscriberMessage {
                        connection_id: *connection_id,
                        backend_id: *backend_id,
                        message_id: msg.id,
                    },
                });
                let payload = searchable_payload_text(&msg.payload);
                if !payload.is_empty() {
                    records.push(SearchRecordSnapshot {
                        field: SearchField::Payload,
                        item_id: msg.id.to_string(),
                        item_label: item_label.clone(),
                        text: payload.clone(),
                        snippet: compact_text(&payload, 160),
                        locator: SearchResultLocator::SubscriberMessage {
                            connection_id: *connection_id,
                            backend_id: *backend_id,
                            message_id: msg.id,
                        },
                    });
                }
            }

            Some(SearchSourceSnapshot {
                id,
                label: format!(
                    "{} {display_id}({connection_name})",
                    t("common.tab_subscriber")
                ),
                generation: state.cache_generation,
                coverage: SearchSourceCoverage::Subscriber {
                    messages: state.messages.len(),
                    max_messages: state.max_messages,
                },
                records,
            })
        }
        _ => None,
    }
}

pub fn search_workspace_ui(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    sources: &[SearchSourceSnapshot],
    actions: &mut Vec<TabAction>,
) {
    render_toolbar(ui, state, sources);
    ui.add_space(4.0);
    render_selected_sources(ui, state, sources, actions);
    ui.separator();

    let results = workspace_results(state, sources).to_vec();
    if let Some(selected) = &state.selected_result
        && !results.iter().any(|result| &result.identity == selected)
        && state.selected_preview.is_none()
    {
        state.selected_result = None;
    }

    egui::Panel::left("search_workspace_results_panel")
        .resizable(true)
        .default_size(360.0)
        .size_range(260.0..=f32::INFINITY)
        .show_inside(ui, |ui| {
            render_results(ui, state, sources, &results);
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        render_preview(ui, state, &results, actions);
    });
}

fn render_toolbar(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    sources: &[SearchSourceSnapshot],
) {
    let before = (state.query.clone(), state.primary, state.secondary);
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .id_salt("search_workspace_query")
                .hint_text(t("search_workspace.query_placeholder"))
                .desired_width((ui.available_width() - 190.0).clamp(160.0, 420.0)),
        );
        egui::ComboBox::from_id_salt("search_workspace_fields")
            .width(116.0)
            .selected_text(t("common.search_fields"))
            .show_ui(ui, |ui| {
                ui.checkbox(&mut state.primary, t("search_workspace.scope_primary"));
                ui.checkbox(&mut state.secondary, t("search_workspace.scope_secondary"));
            });
        egui::ComboBox::from_id_salt("search_workspace_source_picker")
            .width(150.0)
            .selected_text(t("search_workspace.add_source"))
            .show_ui(ui, |ui| {
                for source in sources {
                    if state.selected_sources.contains(&source.id) {
                        continue;
                    }
                    if ui.selectable_label(false, &source.label).clicked() {
                        state.selected_sources.push(source.id.clone());
                        state.cached_results = None;
                    }
                }
                if sources
                    .iter()
                    .all(|source| state.selected_sources.contains(&source.id))
                {
                    ui.weak(t("search_workspace.no_available_sources"));
                }
            });
    });

    if before != (state.query.clone(), state.primary, state.secondary) {
        state.cached_results = None;
        state.selected_result = None;
        state.selected_preview = None;
    }
}

fn render_selected_sources(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    sources: &[SearchSourceSnapshot],
    actions: &mut Vec<TabAction>,
) {
    if state.selected_sources.is_empty() {
        ui.weak(t("search_workspace.select_sources"));
        return;
    }

    let mut remove = Vec::new();
    egui::ScrollArea::horizontal()
        .id_salt("search_workspace_selected_sources")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for source_id in state.selected_sources.clone() {
                    let snapshot = sources.iter().find(|source| source.id == source_id);
                    render_selected_source_chip(ui, &source_id, snapshot, actions, &mut remove);
                }
            });
        });

    if !remove.is_empty() {
        state
            .selected_sources
            .retain(|source_id| !remove.contains(source_id));
        state.cached_results = None;
        if state
            .selected_result
            .as_ref()
            .is_some_and(|selected| remove.contains(&selected.source_id))
        {
            state.selected_result = None;
            state.selected_preview = None;
        }
    }
}

fn render_selected_source_chip(
    ui: &mut egui::Ui,
    source_id: &SearchSourceId,
    snapshot: Option<&SearchSourceSnapshot>,
    actions: &mut Vec<TabAction>,
    remove: &mut Vec<SearchSourceId>,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            let label = snapshot
                .map(|source| source.label.as_str())
                .unwrap_or_else(|| t("search_workspace.source_unavailable"));
            clipped_label(ui, label, SOURCE_CHIP_LABEL_WIDTH);
            if let Some(snapshot) = snapshot {
                clipped_weak_label(ui, &coverage_text(snapshot), SOURCE_CHIP_COVERAGE_WIDTH);
            } else {
                clipped_weak_label(ui, &source_id.fallback_label(), SOURCE_CHIP_FALLBACK_WIDTH);
            }
            if let Some(snapshot) = snapshot
                && let SearchSourceCoverage::Kv {
                    scanning,
                    can_scan_more,
                    ..
                } = snapshot.coverage
            {
                if scanning > 0 {
                    ui.spinner();
                    ui.weak(t("search_workspace.scanning_values"));
                } else if can_scan_more {
                    let label = if has_fetched_values(snapshot) {
                        t("search_workspace.scan_more_values")
                    } else {
                        t("search_workspace.scan_values")
                    };
                    if ui.small_button(label).clicked() {
                        actions.push(TabAction::ScanSearchWorkspaceKvValues {
                            source_id: source_id.clone(),
                        });
                    }
                }
            }
            if ui.small_button("x").clicked() {
                remove.push(source_id.clone());
            }
        });
    });
}

fn clipped_label(ui: &mut egui::Ui, text: &str, width: f32) {
    ui.add_sized(
        [width, ui.spacing().interact_size.y],
        egui::Label::new(text).truncate(),
    )
    .on_hover_text(text);
}

fn clipped_weak_label(ui: &mut egui::Ui, text: &str, width: f32) {
    ui.add_sized(
        [width, ui.spacing().interact_size.y],
        egui::Label::new(egui::RichText::new(text).weak()).truncate(),
    )
    .on_hover_text(text);
}

fn render_results(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    sources: &[SearchSourceSnapshot],
    results: &[SearchWorkspaceResult],
) {
    ui.horizontal(|ui| {
        ui.label(t("search_workspace.results"));
        if state.query.trim().is_empty() {
            ui.weak(t("search_workspace.enter_query"));
        } else {
            let capped = results.len() >= SEARCH_RESULT_LIMIT;
            let suffix = if capped { "+" } else { "" };
            ui.weak(format!(
                "{} {}{}",
                t("common.search_matches"),
                results.len(),
                suffix
            ));
        }
    });

    if state.selected_sources.is_empty() {
        ui.add_space(8.0);
        ui.label(t("search_workspace.empty_sources"));
        return;
    }
    if state.query.trim().is_empty() {
        ui.add_space(8.0);
        ui.label(t("search_workspace.empty_query"));
        return;
    }
    if results.is_empty() {
        ui.add_space(8.0);
        ui.label(t("search_workspace.no_matches"));
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("search_workspace_results")
        .auto_shrink(false)
        .show(ui, |ui| {
            for source_id in state.selected_sources.clone() {
                let group_results = results
                    .iter()
                    .filter(|result| result.identity.source_id == source_id)
                    .collect::<Vec<_>>();
                if group_results.is_empty() {
                    continue;
                }
                render_result_group_header(ui, sources, group_results[0]);
                ui.add_space(2.0);
                for result in group_results {
                    render_result_row(ui, state, result);
                }
                ui.add_space(8.0);
            }
        });
}

fn render_result_group_header(
    ui: &mut egui::Ui,
    sources: &[SearchSourceSnapshot],
    first_result: &SearchWorkspaceResult,
) {
    let source = sources
        .iter()
        .find(|source| source.id == first_result.identity.source_id);
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(&first_result.source_label).strong());
        if let Some(source) = source {
            ui.weak(coverage_text(source));
        }
    });
}

fn render_result_row(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    result: &SearchWorkspaceResult,
) {
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let selected = state
            .selected_result
            .as_ref()
            .is_some_and(|identity| identity == &result.identity);
        let row = format!(
            "{} · {}",
            field_label(result.field),
            compact_label(&result.item_label)
        );
        if ui
            .selectable_label(selected, row)
            .on_hover_text(&result.snippet)
            .clicked()
        {
            state.selected_result = Some(result.identity.clone());
            state.selected_preview = Some(result.clone());
        }
    });
}

fn render_preview(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    results: &[SearchWorkspaceResult],
    actions: &mut Vec<TabAction>,
) {
    let Some(preview) = state.selected_preview.as_ref() else {
        ui.centered_and_justified(|ui| {
            ui.label(t("search_workspace.select_result"));
        });
        return;
    };

    let current = results
        .iter()
        .find(|result| result.identity == preview.identity);
    let active_preview = current.unwrap_or(preview);

    ui.heading(&active_preview.item_label);
    ui.horizontal_wrapped(|ui| {
        ui.label(&active_preview.source_label);
        ui.weak(field_label(active_preview.field));
        if current.is_none() {
            ui.weak(t("search_workspace.stale_result"));
        }
    });
    ui.add_space(4.0);
    if let Some(current) = current {
        if ui.button(t("search_workspace.open_source")).clicked() {
            actions.push(TabAction::NavigateSearchResult {
                locator: current.locator.clone(),
            });
        }
    } else {
        ui.add_enabled(false, egui::Button::new(t("search_workspace.open_source")));
    }
    ui.separator();
    let mut preview_text = active_preview.preview.clone();
    ui.add(
        egui::TextEdit::multiline(&mut preview_text)
            .desired_width(f32::INFINITY)
            .desired_rows(14)
            .code_editor()
            .interactive(false),
    );
}

fn workspace_results<'a>(
    state: &'a mut SearchWorkspaceState,
    sources: &[SearchSourceSnapshot],
) -> &'a [SearchWorkspaceResult] {
    let key = SearchWorkspaceCacheKey {
        query: state.query.trim().to_lowercase(),
        primary: state.primary,
        secondary: state.secondary,
        sources: state
            .selected_sources
            .iter()
            .map(|source_id| {
                let generation = sources
                    .iter()
                    .find(|source| &source.id == source_id)
                    .map(|source| source.generation);
                (source_id.clone(), generation)
            })
            .collect(),
    };

    let needs_refresh = state
        .cached_results
        .as_ref()
        .is_none_or(|(cached_key, _)| *cached_key != key);

    if needs_refresh {
        let mut results = Vec::new();
        if !key.query.is_empty() && (key.primary || key.secondary) {
            for source_id in &state.selected_sources {
                let Some(source) = sources.iter().find(|source| &source.id == source_id) else {
                    continue;
                };
                for record in &source.records {
                    if results.len() >= SEARCH_RESULT_LIMIT {
                        break;
                    }
                    if !field_enabled(record.field, key.primary, key.secondary) {
                        continue;
                    }
                    if matches_query(&record.text, &key.query) {
                        results.push(SearchWorkspaceResult {
                            identity: SearchResultIdentity {
                                source_id: source.id.clone(),
                                generation: source.generation,
                                field: record.field,
                                item_id: record.item_id.clone(),
                            },
                            source_label: source.label.clone(),
                            field: record.field,
                            item_label: record.item_label.clone(),
                            snippet: record.snippet.clone(),
                            preview: record.text.clone(),
                            locator: record.locator.clone(),
                        });
                    }
                }
            }
        }
        state.cached_results = Some((key, results));
    }

    state
        .cached_results
        .as_ref()
        .map(|(_, results)| results.as_slice())
        .unwrap_or(&[])
}

fn field_enabled(field: SearchField, primary: bool, secondary: bool) -> bool {
    if field.is_primary() {
        primary
    } else {
        secondary
    }
}

fn field_label(field: SearchField) -> &'static str {
    match field {
        SearchField::Key => t("search_workspace.field_key"),
        SearchField::Value => t("search_workspace.field_value"),
        SearchField::Subject => t("search_workspace.field_subject"),
        SearchField::Payload => t("search_workspace.field_payload"),
    }
}

fn coverage_text(source: &SearchSourceSnapshot) -> String {
    match source.coverage {
        SearchSourceCoverage::Kv {
            loaded_keys,
            fetched_values,
            ..
        } => format!(
            "{} {loaded_keys} · {} {fetched_values}",
            t("search_workspace.loaded_keys"),
            t("search_workspace.fetched_values")
        ),
        SearchSourceCoverage::Stream { messages } => {
            format!("{} {messages}", t("search_workspace.loaded_messages"))
        }
        SearchSourceCoverage::Subscriber {
            messages,
            max_messages,
        } => format!(
            "{} {messages}/{max_messages}",
            t("search_workspace.retained_messages")
        ),
    }
}

fn has_fetched_values(source: &SearchSourceSnapshot) -> bool {
    matches!(
        source.coverage,
        SearchSourceCoverage::Kv {
            fetched_values,
            ..
        } if fetched_values > 0
    )
}

fn compact_label(label: &str) -> String {
    compact_text(label, 90)
}

pub(crate) fn compact_text(text: &str, max_chars: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let snippet: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{snippet}...")
    } else {
        snippet
    }
}

#[cfg(test)]
mod tests;
