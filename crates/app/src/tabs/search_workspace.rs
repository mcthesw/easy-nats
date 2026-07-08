use eframe::egui;

use crate::format::{self, PayloadFormat};
use crate::i18n::t;

use super::common::SEARCH_RESULT_LIMIT;
use super::types::{
    PreviewFetchState, SearchField, SearchResultKey, SearchResultLocator, SearchSourceCoverage,
    SearchSourceId, SearchSourceKind, SearchSourceSummary, SearchWorkspaceResult,
    SearchWorkspaceState, TabAction, TabKind,
};

mod results;

pub(crate) use results::{SearchWorkspaceBuildStats, append_search_workspace_results};

const SOURCE_CHIP_LABEL_WIDTH: f32 = 190.0;
const SOURCE_CHIP_COVERAGE_WIDTH: f32 = 105.0;
const SOURCE_CHIP_FALLBACK_WIDTH: f32 = 125.0;

pub(crate) fn source_summary_from_tab(tab: &TabKind) -> Option<SearchSourceSummary> {
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
            Some(SearchSourceSummary {
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
                kind: SearchSourceKind::Kv,
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
            Some(SearchSourceSummary {
                id,
                label: format!("{stream_name} ({connection_name})"),
                generation: state.search_generation,
                coverage: SearchSourceCoverage::Stream {
                    messages: state.messages.len(),
                },
                kind: SearchSourceKind::Stream,
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
            Some(SearchSourceSummary {
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
                kind: SearchSourceKind::Subscriber,
            })
        }
        _ => None,
    }
}

pub fn search_workspace_ui(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    sources: &[SearchSourceSummary],
    actions: &mut Vec<TabAction>,
) {
    render_toolbar(ui, state, sources);
    ui.add_space(4.0);
    render_selected_sources(ui, state, sources, actions);
    ui.separator();

    let cached_results = state.cached_results.take();
    let results = cached_results
        .as_ref()
        .map(|(_, results)| results.as_slice())
        .unwrap_or(&[]);
    if let Some(selected) = &state.selected_result
        && !results.iter().any(|result| &result.key == selected)
        && state.selected_preview.is_none()
    {
        state.selected_result = None;
    }

    egui::Panel::left("search_workspace_results_panel")
        .resizable(true)
        .default_size(360.0)
        .size_range(260.0..=f32::INFINITY)
        .show(ui, |ui| {
            render_results(ui, state, sources, results);
        });

    egui::CentralPanel::default().show(ui, |ui| {
        render_preview(ui, state, results, actions);
    });

    state.cached_results = cached_results;
}

fn render_toolbar(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    sources: &[SearchSourceSummary],
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
        state.preview_fetch = PreviewFetchState::Idle;
    }
}

fn render_selected_sources(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    sources: &[SearchSourceSummary],
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
                    let source = sources.iter().find(|source| source.id == source_id);
                    render_selected_source_chip(ui, &source_id, source, actions, &mut remove);
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
            state.preview_fetch = PreviewFetchState::Idle;
        }
    }
}

fn render_selected_source_chip(
    ui: &mut egui::Ui,
    source_id: &SearchSourceId,
    source: Option<&SearchSourceSummary>,
    actions: &mut Vec<TabAction>,
    remove: &mut Vec<SearchSourceId>,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            let label = source
                .map(|source| source.label.as_str())
                .unwrap_or_else(|| t("search_workspace.source_unavailable"));
            clipped_label(ui, label, SOURCE_CHIP_LABEL_WIDTH);
            if let Some(source) = source {
                clipped_weak_label(ui, &coverage_text(source), SOURCE_CHIP_COVERAGE_WIDTH);
            } else {
                clipped_weak_label(ui, &source_id.fallback_label(), SOURCE_CHIP_FALLBACK_WIDTH);
            }
            if let Some(source) = source
                && let SearchSourceCoverage::Kv {
                    scanning,
                    can_scan_more,
                    ..
                } = source.coverage
            {
                if scanning > 0 {
                    ui.spinner();
                    ui.weak(t("search_workspace.scanning_values"));
                } else if can_scan_more {
                    let label = if has_fetched_values(source) {
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
    sources: &[SearchSourceSummary],
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
                let mut group_results = results
                    .iter()
                    .filter(|result| result.key.source_id == source_id);
                let Some(first_result) = group_results.next() else {
                    continue;
                };
                render_result_group_header(ui, sources, first_result);
                ui.add_space(2.0);
                render_result_row(ui, state, first_result);
                for result in group_results {
                    render_result_row(ui, state, result);
                }
                ui.add_space(8.0);
            }
        });
}

fn render_result_group_header(
    ui: &mut egui::Ui,
    sources: &[SearchSourceSummary],
    first_result: &SearchWorkspaceResult,
) {
    let source = sources
        .iter()
        .find(|source| source.id == first_result.key.source_id);
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
            .is_some_and(|identity| identity == &result.key);
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
            state.selected_result = Some(result.key.clone());
            state.selected_preview = Some(result.clone());
            state.preview_fetch = PreviewFetchState::Idle;
        }
    });
}

fn render_preview(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    results: &[SearchWorkspaceResult],
    actions: &mut Vec<TabAction>,
) {
    let Some(preview_key) = state
        .selected_preview
        .as_ref()
        .map(|preview| preview.key.clone())
    else {
        ui.centered_and_justified(|ui| {
            ui.label(t("search_workspace.select_result"));
        });
        return;
    };

    if let Some(active_preview) = results.iter().find(|result| result.key == preview_key) {
        render_current_preview(ui, state, active_preview, actions);
    } else {
        render_stale_preview(ui, state, actions);
    }
}

fn render_current_preview(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    active_preview: &SearchWorkspaceResult,
    actions: &mut Vec<TabAction>,
) {
    render_preview_header(
        ui,
        state,
        actions,
        PreviewHeader {
            item_label: &active_preview.item_label,
            source_label: &active_preview.source_label,
            field: active_preview.field,
            locator: Some(&active_preview.locator),
        },
    );
    match &active_preview.preview_bytes {
        Some(bytes) => {
            state.preview_fetch = PreviewFetchState::Idle;
            render_formatted_preview_bytes(ui, bytes, state.preview_format, &state.query);
        }
        None => {
            render_pending_kv_value_preview(ui, state, active_preview, actions);
        }
    }
}

fn render_stale_preview(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    actions: &mut Vec<TabAction>,
) {
    let Some(snapshot) = state.selected_preview.as_ref() else {
        return;
    };
    let item_label = snapshot.item_label.clone();
    let source_label = snapshot.source_label.clone();
    let field = snapshot.field;
    let has_preview_bytes = snapshot.preview_bytes.is_some();

    render_preview_header(
        ui,
        state,
        actions,
        PreviewHeader {
            item_label: &item_label,
            source_label: &source_label,
            field,
            locator: None,
        },
    );
    state.preview_fetch = PreviewFetchState::Idle;
    if has_preview_bytes {
        let selected_format = state.preview_format;
        let query = state.query.clone();
        if let Some(bytes) = state
            .selected_preview
            .as_ref()
            .and_then(|preview| preview.preview_bytes.as_deref())
        {
            render_formatted_preview_bytes(ui, bytes, selected_format, &query);
        }
    } else {
        ui.weak(t("search_workspace.value_not_found"));
    }
}

struct PreviewHeader<'a> {
    item_label: &'a str,
    source_label: &'a str,
    field: SearchField,
    locator: Option<&'a SearchResultLocator>,
}

fn render_preview_header(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    actions: &mut Vec<TabAction>,
    header: PreviewHeader<'_>,
) {
    ui.heading(header.item_label);
    ui.horizontal_wrapped(|ui| {
        ui.label(header.source_label);
        ui.weak(field_label(header.field));
        if header.locator.is_none() {
            ui.weak(t("search_workspace.stale_result"));
        }
    });
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        render_preview_format_selector(ui, &mut state.preview_format);
        if let Some(locator) = header.locator {
            if ui.button(t("search_workspace.open_source")).clicked() {
                actions.push(TabAction::NavigateSearchResult {
                    locator: locator.clone(),
                });
            }
        } else {
            ui.add_enabled(false, egui::Button::new(t("search_workspace.open_source")));
        }
    });
    ui.separator();
}

/// Renders the preview area for a KV Key match whose value hasn't been
/// fetched yet. Manages the `PreviewFetchState` lifecycle: idle triggers a
/// fetch, loading shows a spinner, failed shows an error with retry.
fn render_pending_kv_value_preview(
    ui: &mut egui::Ui,
    state: &mut SearchWorkspaceState,
    active_preview: &SearchWorkspaceResult,
    actions: &mut Vec<TabAction>,
) {
    let active_key = active_preview.key.clone();
    let active_locator = active_preview.locator.clone();
    // Clone the fetch state so we can read it and then mutate `state`
    // without a borrow conflict.
    let fetch_state = state.preview_fetch.clone();

    match &fetch_state {
        PreviewFetchState::Idle => {
            state.preview_fetch = PreviewFetchState::Loading(active_key.clone());
            emit_fetch_action(&active_locator, actions);
            show_loading(ui);
        }
        PreviewFetchState::Loading(expected) if expected == &active_key => {
            show_loading(ui);
        }
        PreviewFetchState::Loading(_) => {
            // Selection changed to a different key — reset and re-trigger next frame.
            state.preview_fetch = PreviewFetchState::Idle;
            show_loading(ui);
        }
        PreviewFetchState::Failed { key, message } if key == &active_key => {
            let message = message.clone();
            show_error_with_retry(ui, &message, &active_key, &active_locator, state, actions);
        }
        PreviewFetchState::Failed { .. } => {
            // Selection changed — reset and re-trigger next frame.
            state.preview_fetch = PreviewFetchState::Idle;
            show_loading(ui);
        }
    }
}

fn emit_fetch_action(locator: &SearchResultLocator, actions: &mut Vec<TabAction>) {
    if let SearchResultLocator::KvKey {
        connection_id,
        bucket_name,
        key,
    } = locator
    {
        actions.push(TabAction::FetchSearchWorkspaceKvValue {
            source_id: SearchSourceId::Kv {
                connection_id: *connection_id,
                bucket_name: bucket_name.clone(),
            },
            key: key.clone(),
        });
    }
}

fn show_loading(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(t("search_workspace.loading_value"));
    });
}

fn show_error_with_retry(
    ui: &mut egui::Ui,
    message: &str,
    active_key: &SearchResultKey,
    active_locator: &SearchResultLocator,
    state: &mut SearchWorkspaceState,
    actions: &mut Vec<TabAction>,
) {
    ui.colored_label(
        ui.visuals().error_fg_color,
        t("search_workspace.fetch_error"),
    );
    ui.weak(message);
    if ui.button(t("search_workspace.retry")).clicked() {
        state.preview_fetch = PreviewFetchState::Loading(active_key.clone());
        emit_fetch_action(active_locator, actions);
    }
}

fn render_preview_format_selector(ui: &mut egui::Ui, format: &mut PayloadFormat) {
    if !format::READ_ONLY_PREVIEW_FORMATS.contains(format) {
        *format = PayloadFormat::Auto;
    }
    ui.label(t("search_workspace.preview_format"));
    egui::ComboBox::from_id_salt("search_workspace_preview_format")
        .selected_text(format.label())
        .show_ui(ui, |ui| {
            for &choice in format::READ_ONLY_PREVIEW_FORMATS {
                ui.selectable_value(format, choice, choice.label());
            }
        });
}

fn render_formatted_preview_bytes(
    ui: &mut egui::Ui,
    bytes: &[u8],
    selected_format: PayloadFormat,
    query: &str,
) {
    let preview = format::format_read_only_preview(bytes, selected_format);
    let job = format::read_only_preview_layout_job(&preview, ui.style(), query);
    egui::ScrollArea::both()
        .id_salt("search_workspace_preview")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(egui::Label::new(job).selectable(true));
        });
}

#[cfg(test)]
fn active_preview_result<'a>(
    preview: &'a SearchWorkspaceResult,
    results: &'a [SearchWorkspaceResult],
) -> (&'a SearchWorkspaceResult, bool) {
    results
        .iter()
        .find(|result| result.key == preview.key)
        .map_or((preview, true), |current| (current, false))
}

fn field_label(field: SearchField) -> &'static str {
    match field {
        SearchField::Key => t("search_workspace.field_key"),
        SearchField::Value => t("search_workspace.field_value"),
        SearchField::Subject => t("search_workspace.field_subject"),
        SearchField::Payload => t("search_workspace.field_payload"),
    }
}

fn coverage_text(source: &SearchSourceSummary) -> String {
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

fn has_fetched_values(source: &SearchSourceSummary) -> bool {
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
    let mut snippet = String::new();
    let mut written = 0usize;
    let mut truncated = false;

    'words: for word in text.split_whitespace() {
        if !snippet.is_empty() && push_limited_char(&mut snippet, ' ', max_chars, &mut written) {
            truncated = true;
            break;
        }
        for ch in word.chars() {
            if push_limited_char(&mut snippet, ch, max_chars, &mut written) {
                truncated = true;
                break 'words;
            }
        }
    }

    if truncated {
        snippet.push_str("...");
    }
    snippet
}

fn push_limited_char(output: &mut String, ch: char, max_chars: usize, written: &mut usize) -> bool {
    if *written >= max_chars {
        return true;
    }
    output.push(ch);
    *written += 1;
    false
}

#[cfg(test)]
mod tests;
