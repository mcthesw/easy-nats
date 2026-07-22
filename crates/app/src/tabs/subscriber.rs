use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::i18n::t;
use crate::schema::MessageSchemaManager;
use crate::theme::SyntaxPalette;

use super::common::{
    NormalizedSearchQuery, SEARCH_RESULT_LIMIT, SearchStatus, format_timestamp, render_search_row,
    searchable_payload_text, topic_history_text_edit,
};
use super::guard::TabGuard;
use super::subscriber_detail::message_detail_ui;
use super::types::{
    ReceivedMessage, ReplyListStatus, SearchCacheKey, SubjectSubscription, SubscriberState,
    TabAction,
};

#[allow(clippy::too_many_arguments)]
pub fn subscriber_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    backend_id: u64,
    guard: &TabGuard,
    state: &mut SubscriberState,
    backend: &BackendHandle,
    schema_manager: &MessageSchemaManager,
    actions: &mut Vec<TabAction>,
    topic_suggestions: &[&str],
    syntax_palette: SyntaxPalette,
) {
    render_subscription_controls(
        ui,
        connection_id,
        backend_id,
        guard,
        state,
        backend,
        actions,
        topic_suggestions,
    );
    ui.separator();

    // Horizontal split: left message list, right detail
    let panel_id = egui::Id::new(("sub_left_panel", connection_id, backend_id));
    egui::Panel::left(panel_id)
        .resizable(true)
        .default_size(300.0)
        .size_range(200.0..=f32::INFINITY)
        .show(ui, |ui| {
            render_message_list(ui, state);
        });

    egui::CentralPanel::default().show(ui, |ui| {
        if let Some(selected_idx) = state.selected_idx.filter(|idx| *idx < state.messages.len()) {
            message_detail_ui(
                ui,
                connection_id,
                backend_id,
                selected_idx,
                state,
                backend,
                schema_manager,
                syntax_palette,
            );
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(t("subscriber.select_msg"));
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn render_subscription_controls(
    ui: &mut egui::Ui,
    connection_id: u64,
    backend_id: u64,
    guard: &TabGuard,
    state: &mut SubscriberState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
    topic_suggestions: &[&str],
) {
    // Add new subscription input
    let mut do_subscribe = false;
    ui.horizontal(|ui| {
        ui.label(t("subscriber.subject"));
        let input_resp = topic_history_text_edit(
            ui,
            "subscriber_topic_suggestions",
            &mut state.subject_input,
            &mut state.subject_suggestion_idx,
            topic_suggestions,
        );
        let input_id = input_resp.id;

        let can_add = !state.subject_input.trim().is_empty();
        let enter_pressed =
            input_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui
            .add_enabled(can_add, egui::Button::new(t("subscriber.add")))
            .clicked()
            || (enter_pressed && can_add)
        {
            do_subscribe = true;
            ui.memory_mut(|mem| mem.request_focus(input_id));
        }
    });

    if do_subscribe {
        let subject = state.subject_input.trim().to_string();
        if !state.subscriptions.iter().any(|s| s.subject == subject) {
            backend.send(BackendCommand::Subscribe {
                connection_id,
                backend_id,
                subject: subject.clone(),
                cancel: guard.cancellation(),
            });
            state.subscriptions.push(SubjectSubscription {
                subject: subject.clone(),
                active: true,
            });
            actions.push(TabAction::RecordTopic { topic: subject });
        }
        state.subject_input.clear();
        state.subject_suggestion_idx = None;
    }

    // Active subscriptions list
    if !state.subscriptions.is_empty() {
        ui.add_space(4.0);
        ui.label(t("subscriber.subscriptions"));
        let mut to_remove = Vec::new();
        for (i, sub) in state.subscriptions.iter().enumerate() {
            ui.horizontal(|ui| {
                let color = if sub.active {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::GRAY
                };
                ui.colored_label(color, "●");
                ui.label(&sub.subject);
                if sub.active && ui.small_button(t("subscriber.unsubscribe")).clicked() {
                    to_remove.push(i);
                }
            });
        }
        for i in to_remove.into_iter().rev() {
            let sub = &state.subscriptions[i];
            backend.send(BackendCommand::Unsubscribe {
                connection_id,
                backend_id,
                subject: sub.subject.clone(),
            });
            state.subscriptions.remove(i);
        }
    }
}

fn render_message_list(ui: &mut egui::Ui, state: &mut SubscriberState) {
    ui.horizontal(|ui| {
        ui.label(format!(
            "{} {} / {}",
            t("subscriber.msg_count"),
            state.messages.len(),
            state.max_messages
        ));

        // Subject filter dropdown
        if state.subscriptions.len() > 1 {
            let filter_label = state
                .subject_filter
                .as_deref()
                .unwrap_or(t("subscriber.filter_all"));
            egui::ComboBox::from_id_salt("sub_subject_filter")
                .selected_text(filter_label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            &mut state.subject_filter,
                            None,
                            t("subscriber.filter_all"),
                        )
                        .changed()
                    {
                        state.selected_idx = None;
                        state.cached_filtered = None;
                    }
                    for sub in &state.subscriptions {
                        let val = Some(sub.subject.clone());
                        if ui
                            .selectable_value(&mut state.subject_filter, val, &sub.subject)
                            .changed()
                        {
                            state.selected_idx = None;
                            state.cached_filtered = None;
                        }
                    }
                });
        }

        if ui.small_button(t("subscriber.clear")).clicked() {
            state.clear_messages();
        }
    });

    ui.add_space(4.0);
    ui.label(t("subscriber.messages"));
    let status = subscriber_search_status(state);
    if render_search_row(
        ui,
        "subscriber_search",
        &mut state.search,
        t("subscriber.search_placeholder"),
        t("subscriber.search_scope_subject"),
        t("subscriber.search_scope_payload"),
    ) {
        state.cached_filtered = None;
    }
    if state.search.is_active() {
        ui.horizontal_wrapped(|ui| {
            if let Some(text) = status.text() {
                ui.weak(text);
            }
            ui.weak(format!("· {}", t("subscriber.search_buffer_only")));
        });
    }

    let mut next_selected_idx = state.selected_idx;
    let filtered = filtered_rows(state);
    if filtered.is_empty() {
        ui.label(if state.messages.is_empty() {
            t("subscriber.no_messages")
        } else {
            t("common.search_no_matches")
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("sub_msg_list")
        .stick_to_bottom(true)
        .auto_shrink(false)
        .show_rows(ui, 36.0, filtered.len(), |ui, row_range| {
            for (idx, time, subject, reply_status) in &filtered[row_range] {
                let selected = next_selected_idx == Some(*idx);
                let visuals = ui.visuals();
                let time_color = visuals.weak_text_color();
                let subj_color = if selected {
                    visuals.strong_text_color()
                } else {
                    visuals.text_color()
                };
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    time,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(11.0),
                        color: time_color,
                        ..Default::default()
                    },
                );
                if let Some(reply_status) = reply_status {
                    let reply_color = match reply_status {
                        ReplyListStatus::Failed => visuals.error_fg_color,
                        _ => time_color,
                    };
                    job.append(
                        &format!("  {}", t(reply_status.label_key())),
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::proportional(11.0),
                            color: reply_color,
                            ..Default::default()
                        },
                    );
                }
                job.append(
                    &format!("\n{subject}"),
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(13.0),
                        color: subj_color,
                        ..Default::default()
                    },
                );
                if ui.selectable_label(selected, job).clicked() {
                    next_selected_idx = Some(*idx);
                }
            }
        });
    state.selected_idx = next_selected_idx;
}

fn subscriber_search_status(state: &mut SubscriberState) -> SearchStatus {
    if !state.search.is_active() {
        return SearchStatus::Inactive;
    }
    let rows = filtered_rows(state);
    SearchStatus::Showing {
        matches: rows.len(),
        capped: rows.len() >= SEARCH_RESULT_LIMIT,
    }
}

fn filtered_rows(
    state: &mut SubscriberState,
) -> &[(usize, String, String, Option<ReplyListStatus>)] {
    let search_key = SearchCacheKey::from_state(&state.search);
    let needs_refresh = match &state.cached_filtered {
        Some((generation, filter, cached_search, _)) => {
            *generation != state.cache_generation
                || *filter != state.subject_filter
                || *cached_search != search_key
        }
        None => true,
    };

    if needs_refresh {
        let subject_filter = state.subject_filter.clone();
        let search_active = state.search.is_active();
        let query = search_active.then(|| {
            NormalizedSearchQuery::new(&search_key.query)
                .expect("active search has a normalized query")
        });
        let rows = state
            .messages
            .iter()
            .enumerate()
            .filter(|(_, message)| {
                subject_filter
                    .as_ref()
                    .is_none_or(|filter| message.subject == *filter)
                    && query.as_ref().is_none_or(|query| {
                        subscriber_message_matches(message, &state.search, query)
                    })
            })
            .take(SEARCH_RESULT_LIMIT)
            .map(|(idx, message)| {
                (
                    idx,
                    format_timestamp(message.timestamp),
                    message.subject.clone(),
                    message.reply_list_status(),
                )
            })
            .collect();
        state.cached_filtered = Some((state.cache_generation, subject_filter, search_key, rows));
    }

    state
        .cached_filtered
        .as_ref()
        .map(|(_, _, _, rows)| rows.as_slice())
        .unwrap_or(&[])
}

fn subscriber_message_matches(
    message: &ReceivedMessage,
    search: &super::types::ScopedSearchState,
    query: &NormalizedSearchQuery,
) -> bool {
    query.matches_scoped(
        search.primary,
        &message.subject,
        search.secondary,
        |query| query.matches(&searchable_payload_text(&message.payload)),
    )
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;
    use crate::tabs::types::ReplyState;

    fn make_msg(subject: &str) -> ReceivedMessage {
        make_msg_with_payload(subject, b"")
    }

    fn make_msg_with_payload(subject: &str, payload: &[u8]) -> ReceivedMessage {
        ReceivedMessage::new(
            subject.to_string(),
            None,
            Vec::new(),
            payload.to_vec(),
            SystemTime::now(),
        )
    }

    fn make_replyable_msg(subject: &str, reply_to: &str) -> ReceivedMessage {
        ReceivedMessage::new(
            subject.to_string(),
            Some(reply_to.to_string()),
            Vec::new(),
            Vec::new(),
            SystemTime::now(),
        )
    }

    #[test]
    fn ring_buffer_evicts_oldest_at_capacity() {
        let mut state = SubscriberState {
            max_messages: 2,
            ..Default::default()
        };
        state.push_messages([make_msg("a"), make_msg("b"), make_msg("c")]);
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].subject, "b");
        assert_eq!(state.messages[1].subject, "c");
    }

    #[test]
    fn cache_is_reused_until_state_changes() {
        let mut state = SubscriberState::default();
        state.push_messages([make_msg("alpha"), make_msg("beta")]);

        let first = filtered_rows(&mut state).to_vec();
        assert_eq!(
            state.cached_filtered.as_ref().map(|cached| cached.0),
            Some(1)
        );

        let second = filtered_rows(&mut state).to_vec();
        assert_eq!(first, second);
        assert_eq!(
            state.cached_filtered.as_ref().map(|cached| cached.0),
            Some(1)
        );

        state.subject_filter = Some("beta".to_string());
        state.cached_filtered = None;
        let filtered = filtered_rows(&mut state).to_vec();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, 1);
        assert_eq!(filtered[0].2, "beta");
    }

    #[test]
    fn search_filters_by_subject_or_payload() {
        let mut state = SubscriberState::default();
        state.push_messages([
            make_msg_with_payload("orders.created", b"balance: 10"),
            make_msg_with_payload("payments.updated", b"balance: 42"),
        ]);

        state.search.query = "payments".to_string();
        state.search.primary = true;
        state.search.secondary = false;
        let by_subject = filtered_rows(&mut state).to_vec();
        assert_eq!(by_subject.len(), 1);
        assert_eq!(by_subject[0].2, "payments.updated");

        state.search.query = "42".to_string();
        state.search.primary = false;
        state.search.secondary = true;
        state.cached_filtered = None;
        let by_payload = filtered_rows(&mut state).to_vec();
        assert_eq!(by_payload.len(), 1);
        assert_eq!(by_payload[0].2, "payments.updated");
    }

    #[test]
    fn clear_messages_resets_selection_and_cache() {
        let mut state = SubscriberState::default();
        state.push_messages([make_msg("alpha")]);
        state.selected_idx = Some(0);
        let _ = filtered_rows(&mut state);

        state.clear_messages();

        assert!(state.messages.is_empty());
        assert!(state.in_flight_replies.is_empty());
        assert_eq!(state.selected_idx, None);
        assert!(state.cached_filtered.is_none());
        assert_eq!(state.cache_generation, 2);
    }

    #[test]
    fn selected_index_tracks_source_messages_after_eviction() {
        let mut state = SubscriberState {
            max_messages: 2,
            ..Default::default()
        };
        state.push_messages([make_msg("alpha"), make_msg("beta")]);
        state.selected_idx = Some(1);

        state.push_messages([make_msg("gamma")]);

        assert_eq!(state.selected_idx, Some(0));
        assert_eq!(state.messages[0].subject, "beta");
        assert_eq!(state.messages[1].subject, "gamma");
    }

    #[test]
    fn batch_push_invalidates_cache_once() {
        let mut state = SubscriberState::default();

        state.push_messages([make_msg("alpha"), make_msg("beta")]);

        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].id, 1);
        assert_eq!(state.messages[1].id, 2);
        assert_eq!(state.cache_generation, 1);
        assert!(state.cached_filtered.is_none());
    }

    #[test]
    fn batch_push_preserves_eviction_and_selected_index_behavior() {
        let mut state = SubscriberState {
            max_messages: 3,
            ..Default::default()
        };
        state.push_messages([make_msg("alpha"), make_msg("beta"), make_msg("gamma")]);
        state.selected_idx = Some(2);

        state.push_messages([make_msg("delta"), make_msg("epsilon")]);

        let subjects = state
            .messages
            .iter()
            .map(|message| message.subject.as_str())
            .collect::<Vec<_>>();
        assert_eq!(subjects, vec!["gamma", "delta", "epsilon"]);
        assert_eq!(state.selected_idx, Some(0));
        assert_eq!(state.cache_generation, 2);
    }

    #[test]
    fn replyable_messages_start_with_reply_state_and_draft() {
        let msg = make_replyable_msg("orders.created", "_INBOX.1");

        assert_eq!(msg.reply_state, Some(ReplyState::Replyable));
        assert!(msg.reply_draft.is_some());
        assert_eq!(msg.reply_list_status(), Some(ReplyListStatus::Replyable));
    }

    #[test]
    fn reply_success_updates_original_message() {
        let mut state = SubscriberState::default();
        state.push_messages([make_replyable_msg("orders.created", "_INBOX.1")]);
        let message_id = state.messages[0].id;

        let reply_id = state.begin_reply(message_id).expect("message is replyable");

        assert_eq!(
            state.messages[0].reply_state,
            Some(ReplyState::Sending { reply_id })
        );
        assert_eq!(state.in_flight_replies.get(&reply_id), Some(&message_id));

        state.apply_reply_success(reply_id);

        assert_eq!(state.messages[0].reply_state, Some(ReplyState::Replied));
        assert!(state.in_flight_replies.is_empty());
    }

    #[test]
    fn replied_messages_cannot_start_second_reply_but_failed_replies_can_retry() {
        let mut state = SubscriberState::default();
        state.push_messages([make_replyable_msg("orders.created", "_INBOX.1")]);
        let message_id = state.messages[0].id;

        let reply_id = state.begin_reply(message_id).expect("message is replyable");
        state.apply_reply_success(reply_id);

        assert!(state.begin_reply(message_id).is_none());

        state.push_messages([make_replyable_msg("orders.updated", "_INBOX.2")]);
        let retry_message_id = state.messages[1].id;
        let failed_reply_id = state
            .begin_reply(retry_message_id)
            .expect("message is replyable");
        state.apply_reply_failure(failed_reply_id, "publish failed".to_string());

        assert!(state.begin_reply(retry_message_id).is_some());
    }

    #[test]
    fn evicting_message_forgets_in_flight_reply() {
        let mut state = SubscriberState {
            max_messages: 1,
            ..Default::default()
        };
        state.push_messages([make_replyable_msg("orders.created", "_INBOX.1")]);
        let message_id = state.messages[0].id;
        let reply_id = state.begin_reply(message_id).expect("message is replyable");

        state.push_messages([make_msg("orders.updated")]);

        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].subject, "orders.updated");
        assert!(!state.in_flight_replies.contains_key(&reply_id));
    }
}
