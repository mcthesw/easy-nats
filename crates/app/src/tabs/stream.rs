use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle, StreamInfo, StreamMessageInfo};
use std::time::{Duration, SystemTime};

use crate::format::{self, PayloadFormat};
use crate::i18n::t;
use crate::schema::MessageSchemaManager;

use super::common::{
    NormalizedSearchQuery, SEARCH_RESULT_LIMIT, SearchStatus, auto_refresh_ui, format_bytes,
    render_search_row, searchable_payload_text,
};
use super::stream_consumers::render_consumers;
use super::types::{SearchCacheKey, StreamState, TabAction};

pub fn stream_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    state: &mut StreamState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
    schema_manager: &MessageSchemaManager,
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
                            connection_id,
                            msg,
                            &mut state.payload_format,
                            &mut state.proto_view,
                            schema_manager,
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
    if let Some(info) = &state.info
        && info.retention.to_lowercase().contains("work")
    {
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::YELLOW, "⚠");
            ui.label(t("stream.workqueue_hint"));
        });
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
    let status = stream_search_status(state);
    if render_search_row(
        ui,
        "stream_search",
        &mut state.search,
        t("stream.search_placeholder"),
        t("stream.search_scope_subject"),
        t("stream.search_scope_payload"),
    ) {
        state.cached_filtered = None;
    }
    if state.search.is_active() {
        ui.horizontal_wrapped(|ui| {
            if let Some(text) = status.text() {
                ui.weak(text);
            }
            ui.weak(format!("· {}", t("stream.search_loaded_only")));
        });
    }

    let mut next_selected = state.selected_msg;
    let rows = filtered_stream_rows(state).to_vec();
    egui::ScrollArea::vertical()
        .id_salt("stream_msg_list")
        .auto_shrink(false)
        .show(ui, |ui| {
            if state.messages.is_empty() {
                ui.label(t("stream.no_messages"));
            } else if rows.is_empty() {
                ui.label(t("common.search_no_matches"));
            } else {
                for (idx, label) in rows {
                    let selected = next_selected == Some(idx);
                    if ui.selectable_label(selected, &label).clicked() {
                        next_selected = Some(idx);
                    }
                }
            }
        });
    state.selected_msg = next_selected;
}

fn stream_search_status(state: &mut StreamState) -> SearchStatus {
    if !state.search.is_active() {
        return SearchStatus::Inactive;
    }
    let rows = filtered_stream_rows(state);
    SearchStatus::Showing {
        matches: rows.len(),
        capped: rows.len() >= SEARCH_RESULT_LIMIT,
    }
}

fn filtered_stream_rows(state: &mut StreamState) -> &[(usize, String)] {
    let cache_key = SearchCacheKey::from_state(&state.search);
    let needs_refresh = match &state.cached_filtered {
        Some((generation, cached_key, _)) => {
            *generation != state.search_generation || *cached_key != cache_key
        }
        None => true,
    };

    if needs_refresh {
        let search_active = state.search.is_active();
        let query = search_active.then(|| {
            NormalizedSearchQuery::new(&cache_key.query)
                .expect("active search has a normalized query")
        });
        let rows = state
            .messages
            .iter()
            .enumerate()
            .filter(|(_, msg)| {
                query
                    .as_ref()
                    .is_none_or(|query| stream_message_matches(msg, &state.search, query))
            })
            .take(SEARCH_RESULT_LIMIT)
            .map(|(idx, msg)| (idx, stream_message_label(msg)))
            .collect();
        state.cached_filtered = Some((state.search_generation, cache_key, rows));
    }

    state
        .cached_filtered
        .as_ref()
        .map(|(_, _, rows)| rows.as_slice())
        .unwrap_or(&[])
}

fn stream_message_matches(
    msg: &StreamMessageInfo,
    search: &super::types::ScopedSearchState,
    query: &NormalizedSearchQuery,
) -> bool {
    query.matches_scoped(search.primary, &msg.subject, search.secondary, |query| {
        query.matches(&searchable_payload_text(&msg.payload))
    })
}

fn stream_message_label(msg: &StreamMessageInfo) -> String {
    let seq = msg.sequence;
    let subject = msg.subject.as_str();
    let time_str = format_rfc3339_short(&msg.time).unwrap_or_default();
    if time_str.is_empty() {
        format!("#{seq} {subject}")
    } else {
        format!("#{seq} {time_str} {subject}")
    }
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
        && let Some(subject) = state.messages.get(idx).map(|msg| msg.subject.as_str())
        && is_publishable_subject(subject)
    {
        return subject.to_string();
    }

    let filter = state.subject_filter.trim();
    if is_publishable_subject(filter) {
        return filter.to_string();
    }

    if let Some(info) = &state.info
        && let Some(subject) = info
            .subjects
            .iter()
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
                && ui
                    .button(format!("{} #{}", t("stream.delete_msg"), msg.sequence))
                    .clicked()
            {
                backend.send(BackendCommand::DeleteStreamMessage {
                    connection_id,
                    stream: stream_name.to_string(),
                    sequence: msg.sequence,
                });
            }
        });
}

fn stream_info_panel(ui: &mut egui::Ui, info: &StreamInfo) {
    egui::Grid::new("stream_info_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in [
                (t("stream.name"), info.name.clone()),
                (t("stream.subjects"), info.subjects.join(", ")),
                (t("stream.storage"), info.storage.clone()),
                (t("stream.retention"), info.retention.clone()),
                (t("stream.msg_count"), info.messages.to_string()),
                (t("stream.bytes"), format_bytes(info.bytes)),
                (t("stream.consumers"), info.consumer_count.to_string()),
            ] {
                ui.label(label);
                ui.label(value);
                ui.end_row();
            }
        });
}

fn stream_message_detail(
    ui: &mut egui::Ui,
    connection_id: u64,
    msg: &StreamMessageInfo,
    payload_format: &mut PayloadFormat,
    proto_view: &mut crate::proto::ProtoViewState,
    schema_manager: &MessageSchemaManager,
) {
    ui.horizontal(|ui| {
        ui.label(t("stream.msg_detail"));
        format::format_selector(ui, "stream_msg_fmt", payload_format);
    });

    egui::Grid::new("stream_msg_detail_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("stream.msg_sequence"));
            ui.label(msg.sequence.to_string());
            ui.end_row();

            ui.label(t("stream.msg_subject"));
            ui.label(&msg.subject);
            ui.end_row();

            if !msg.time.is_empty() {
                ui.label(t("stream.msg_time"));
                ui.label(&msg.time);
                ui.end_row();
            }
        });

    if !msg.headers.is_empty() {
        ui.add_space(4.0);
        ui.label(t("stream.msg_headers"));
        egui::Grid::new("stream_msg_headers")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for (name, value) in &msg.headers {
                    ui.label(name);
                    ui.label(value);
                    ui.end_row();
                }
            });
    }

    ui.add_space(4.0);
    ui.label(t("stream.msg_payload"));
    if !msg.subject.is_empty() {
        format::render_payload_with_schema(
            ui,
            &msg.payload,
            *payload_format,
            "stream_proto",
            proto_view,
            format::SchemaRenderContext {
                manager: schema_manager,
                connection_id,
                subject: &msg.subject,
            },
        );
    } else {
        format::render_payload_with_proto(
            ui,
            &msg.payload,
            *payload_format,
            "stream_proto",
            proto_view,
            schema_manager,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream_msg(subject: &str, payload: &[u8]) -> StreamMessageInfo {
        StreamMessageInfo {
            sequence: 1,
            subject: subject.to_string(),
            payload: payload.to_vec(),
            headers: Vec::new(),
            time: String::new(),
        }
    }

    #[test]
    fn inactive_stream_search_lists_loaded_messages_without_query() {
        let mut state = StreamState {
            messages: vec![stream_msg("orders.created", b"balance: 42")],
            ..Default::default()
        };

        let rows = filtered_stream_rows(&mut state).to_vec();

        assert_eq!(rows, vec![(0, "#1 orders.created".to_string())]);
    }
}
