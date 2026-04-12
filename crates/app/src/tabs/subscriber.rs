use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format::{self, PayloadFormat};
use crate::ui_strings as S;

use super::common::{format_timestamp, payload_preview};
use super::types::{ReceivedMessage, SubscriberState};

pub fn subscriber_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut SubscriberState,
    backend: &BackendHandle,
) {
    ui.horizontal(|ui| {
        ui.label(S::SUBSCRIBER_SUBJECT);
        ui.add_enabled(
            !state.subscribed,
            egui::TextEdit::singleline(&mut state.subject),
        );
        let can_toggle = !state.subject.trim().is_empty();
        if state.subscribed {
            if ui
                .add_enabled(true, egui::Button::new(S::SUBSCRIBER_UNSUBSCRIBE))
                .clicked()
            {
                backend.send(BackendCommand::Unsubscribe {
                    connection_id,
                    subject: state.subject.clone(),
                });
                state.subscribed = false;
            }
        } else if ui
            .add_enabled(can_toggle, egui::Button::new(S::SUBSCRIBER_SUBSCRIBE))
            .clicked()
        {
            backend.send(BackendCommand::Subscribe {
                connection_id,
                subject: state.subject.clone(),
            });
            state.subscribed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label(format!(
            "{} {} / {}",
            S::SUBSCRIBER_MSG_COUNT,
            state.messages.len(),
            state.max_messages
        ));
        if ui.small_button(S::SUBSCRIBER_CLEAR).clicked() {
            state.messages.clear();
            state.selected_idx = None;
        }
    });

    ui.add_space(4.0);
    ui.separator();
    let list_height = (ui.available_height() * 0.5).max(100.0);
    ui.label(S::SUBSCRIBER_MESSAGES);
    egui::ScrollArea::vertical()
        .id_salt("sub_msg_list")
        .max_height(list_height)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if state.messages.is_empty() {
                ui.label(S::SUBSCRIBER_NO_MESSAGES);
            } else {
                for (idx, msg) in state.messages.iter().enumerate() {
                    let label = format!(
                        "[{}] {} — {}",
                        format_timestamp(msg.timestamp),
                        msg.subject,
                        payload_preview(&msg.payload, 80)
                    );
                    if ui
                        .selectable_label(state.selected_idx == Some(idx), label)
                        .clicked()
                    {
                        state.selected_idx = Some(idx);
                    }
                }
            }
        });

    ui.add_space(4.0);
    ui.separator();
    if let Some(idx) = state.selected_idx
        && let Some(msg) = state.messages.get(idx)
    {
        message_detail_ui(ui, msg, &mut state.payload_format);
    } else {
        ui.label(S::SUBSCRIBER_SELECT_MSG);
    }
}

fn message_detail_ui(ui: &mut egui::Ui, msg: &ReceivedMessage, payload_format: &mut PayloadFormat) {
    ui.horizontal(|ui| {
        ui.label(S::SUBSCRIBER_DETAIL);
        format::format_selector(ui, "sub_detail_fmt", payload_format);
    });

    egui::Grid::new("msg_detail_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(S::SUBSCRIBER_DETAIL_SUBJECT);
            ui.label(&msg.subject);
            ui.end_row();

            if let Some(reply) = &msg.reply {
                ui.label(S::SUBSCRIBER_DETAIL_REPLY);
                ui.label(reply);
                ui.end_row();
            }

            ui.label(S::SUBSCRIBER_DETAIL_TIMESTAMP);
            ui.label(format_timestamp(msg.timestamp));
            ui.end_row();

            ui.label(S::SUBSCRIBER_DETAIL_SIZE);
            ui.label(format!("{} bytes", msg.payload.len()));
            ui.end_row();
        });

    if !msg.headers.is_empty() {
        ui.add_space(4.0);
        ui.label(S::SUBSCRIBER_DETAIL_HEADERS);
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
    ui.label(S::SUBSCRIBER_DETAIL_PAYLOAD);
    egui::ScrollArea::vertical()
        .id_salt("msg_detail_payload")
        .max_height(200.0)
        .show(ui, |ui| {
            format::render_payload(ui, &msg.payload, *payload_format);
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
