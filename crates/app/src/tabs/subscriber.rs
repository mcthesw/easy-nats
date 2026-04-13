use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format::{self, PayloadFormat};
use crate::i18n::t;
use crate::proto::ProtoSchemaManager;

use super::common::format_timestamp;
use super::types::{ReceivedMessage, SubjectSubscription, SubscriberState};

pub fn subscriber_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut SubscriberState,
    backend: &BackendHandle,
    proto_manager: &ProtoSchemaManager,
) {
    render_subscription_controls(ui, connection_id, state, backend);
    ui.separator();

    // Horizontal split: left message list, right detail
    let panel_id = egui::Id::new(("sub_left_panel", connection_id));
    egui::Panel::left(panel_id)
        .resizable(true)
        .default_size(300.0)
        .size_range(200.0..=f32::INFINITY)
        .show_inside(ui, |ui| {
            render_message_list(ui, state);
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        let selected_msg = state
            .selected_idx
            .and_then(|idx| filtered_messages(state).get(idx).cloned().cloned());
        if let Some(msg) = &selected_msg {
            message_detail_ui(
                ui,
                msg,
                &mut state.payload_format,
                &mut state.proto_view,
                proto_manager,
            );
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(t("subscriber.select_msg"));
            });
        }
    });
}

fn render_subscription_controls(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut SubscriberState,
    backend: &BackendHandle,
) {
    // Add new subscription input
    ui.horizontal(|ui| {
        ui.label(t("subscriber.subject"));
        ui.text_edit_singleline(&mut state.subject_input);
        let can_add = !state.subject_input.trim().is_empty();
        if ui
            .add_enabled(can_add, egui::Button::new(t("subscriber.add")))
            .clicked()
        {
            let subject = state.subject_input.trim().to_string();
            // Avoid duplicate subscriptions
            if !state.subscriptions.iter().any(|s| s.subject == subject) {
                backend.send(BackendCommand::Subscribe {
                    connection_id,
                    subject: subject.clone(),
                });
                state.subscriptions.push(SubjectSubscription {
                    subject,
                    active: true,
                });
            }
            state.subject_input.clear();
        }
    });

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
                    }
                    for sub in &state.subscriptions {
                        let val = Some(sub.subject.clone());
                        if ui
                            .selectable_value(&mut state.subject_filter, val, &sub.subject)
                            .changed()
                        {
                            state.selected_idx = None;
                        }
                    }
                });
        }

        if ui.small_button(t("subscriber.clear")).clicked() {
            state.messages.clear();
            state.selected_idx = None;
        }
    });

    ui.add_space(4.0);
    ui.label(t("subscriber.messages"));

    // Collect filtered message indices to avoid borrow conflicts
    let filtered: Vec<(usize, String, String)> = {
        let msgs = filtered_messages(state);
        msgs.iter()
            .enumerate()
            .map(|(idx, msg)| {
                let time = format_timestamp(msg.timestamp);
                let subject = msg.subject.clone();
                (idx, time, subject)
            })
            .collect()
    };

    egui::ScrollArea::vertical()
        .id_salt("sub_msg_list")
        .stick_to_bottom(true)
        .auto_shrink(false)
        .show(ui, |ui| {
            if filtered.is_empty() {
                ui.label(t("subscriber.no_messages"));
            } else {
                for (idx, time, subject) in &filtered {
                    let selected = state.selected_idx == Some(*idx);
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
                        state.selected_idx = Some(*idx);
                    }
                }
            }
        });
}

fn filtered_messages(state: &SubscriberState) -> Vec<&ReceivedMessage> {
    match &state.subject_filter {
        Some(filter) => state
            .messages
            .iter()
            .filter(|m| m.subject == *filter)
            .collect(),
        None => state.messages.iter().collect(),
    }
}

fn message_detail_ui(
    ui: &mut egui::Ui,
    msg: &ReceivedMessage,
    payload_format: &mut PayloadFormat,
    proto_view: &mut crate::proto::ProtoViewState,
    proto_manager: &ProtoSchemaManager,
) {
    ui.horizontal(|ui| {
        ui.label(t("subscriber.detail"));
        format::format_selector(ui, "sub_detail_fmt", payload_format);
    });

    egui::Grid::new("msg_detail_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("subscriber.detail_subject"));
            ui.label(&msg.subject);
            ui.end_row();

            if let Some(reply) = &msg.reply {
                ui.label(t("subscriber.detail_reply"));
                ui.label(reply);
                ui.end_row();
            }

            ui.label(t("subscriber.detail_timestamp"));
            ui.label(format_timestamp(msg.timestamp));
            ui.end_row();

            ui.label(t("subscriber.detail_size"));
            ui.label(format!("{} bytes", msg.payload.len()));
            ui.end_row();
        });

    if !msg.headers.is_empty() {
        ui.add_space(4.0);
        ui.label(t("subscriber.detail_headers"));
        egui::Grid::new("msg_detail_headers")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for (k, v) in &msg.headers {
                    ui.label(k);
                    ui.label(v);
                    ui.end_row();
                }
            });
    }

    ui.add_space(4.0);
    ui.label(t("subscriber.detail_payload"));
    egui::ScrollArea::vertical()
        .id_salt("msg_detail_payload")
        .show(ui, |ui| {
            format::render_payload_with_proto(
                ui,
                &msg.payload,
                *payload_format,
                "sub_proto",
                proto_view,
                proto_manager,
            );
        });
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    fn make_msg(subject: &str) -> ReceivedMessage {
        ReceivedMessage {
            subject: subject.to_string(),
            reply: None,
            headers: Vec::new(),
            payload: Vec::new(),
            timestamp: SystemTime::now(),
        }
    }

    #[test]
    fn ring_buffer_evicts_oldest_at_capacity() {
        let mut state = SubscriberState {
            max_messages: 2,
            ..Default::default()
        };
        state.push_message(make_msg("a"));
        state.push_message(make_msg("b"));
        state.push_message(make_msg("c"));
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].subject, "b");
        assert_eq!(state.messages[1].subject, "c");
    }
}
