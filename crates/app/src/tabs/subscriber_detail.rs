use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format;
use crate::i18n::t;
use crate::schema::{MessageSchemaManager, PayloadInputFormat};

use super::common::{collect_headers, format_timestamp, payload_input_format_selector};
use super::types::{ReceivedMessage, ReplyDraft, ReplyState, SubscriberState};

struct PendingReply {
    message_id: u64,
    reply_to: String,
    subject: String,
    payload_text: String,
    payload_input_format: PayloadInputFormat,
    headers: Option<Vec<(String, String)>>,
}

pub(super) fn message_detail_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    backend_id: u64,
    selected_idx: usize,
    state: &mut SubscriberState,
    backend: &BackendHandle,
    schema_manager: &MessageSchemaManager,
) {
    let mut pending_reply = None;
    {
        let messages = &mut state.messages;
        let payload_format = &mut state.payload_format;
        let proto_view = &mut state.proto_view;
        let Some(msg) = messages.get_mut(selected_idx) else {
            return;
        };

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
            .max_height(220.0)
            .show(ui, |ui| {
                format::render_payload_with_schema(
                    ui,
                    &msg.payload,
                    *payload_format,
                    "sub_proto",
                    proto_view,
                    format::SchemaRenderContext {
                        manager: schema_manager,
                        connection_id,
                        subject: &msg.subject,
                    },
                );
            });

        render_reply_composer(ui, connection_id, msg, schema_manager, &mut pending_reply);
    }

    if let Some(reply) = pending_reply {
        let outgoing = schema_manager.prepare_outgoing_with_input_format(
            connection_id,
            &reply.subject,
            &reply.payload_text,
            reply.payload_input_format,
        );
        if outgoing.can_send
            && let Some(reply_id) = state.begin_reply(reply.message_id)
        {
            backend.send(BackendCommand::Reply {
                connection_id,
                backend_id,
                reply_id,
                reply_to: reply.reply_to,
                payload: outgoing.payload,
                headers: reply.headers,
            });
        }
    }
}

fn render_reply_composer(
    ui: &mut egui::Ui,
    connection_id: u64,
    msg: &mut ReceivedMessage,
    schema_manager: &MessageSchemaManager,
    pending_reply: &mut Option<PendingReply>,
) {
    let Some(reply_to) = msg.reply.clone() else {
        return;
    };
    if msg.reply_draft.is_none() {
        msg.reply_draft = Some(ReplyDraft::default());
    }

    let reply_state = msg.reply_state.clone().unwrap_or(ReplyState::Replyable);
    let reply_blocked = matches!(
        reply_state,
        ReplyState::Sending { .. } | ReplyState::Replied
    );

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(t("subscriber.reply"));
        ui.weak(t(reply_status_label_key(&reply_state)));
    });

    if let ReplyState::Failed(message) = &reply_state {
        ui.colored_label(
            ui.visuals().error_fg_color,
            format!("{}: {message}", t("subscriber.reply_failed_message")),
        );
    }

    let draft = msg
        .reply_draft
        .as_mut()
        .expect("replyable messages have drafts");
    let outgoing_preview = schema_manager.prepare_outgoing_with_input_format(
        connection_id,
        &msg.subject,
        &draft.payload,
        draft.payload_input_format,
    );

    ui.horizontal(|ui| {
        ui.label(t("subscriber.reply_payload"));
        ui.label(t("common.payload_input_format"));
        payload_input_format_selector(
            ui,
            &format!("subscriber_reply_payload_fmt_{}", msg.id),
            &mut draft.payload_input_format,
        );
    });
    egui::ScrollArea::vertical()
        .id_salt(("subscriber_reply_payload", msg.id))
        .max_height(140.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut draft.payload)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4)
                    .code_editor(),
            );
        });
    if let Some(status) = &outgoing_preview.status {
        format::render_schema_status(ui, status);
    }

    render_reply_headers(ui, msg.id, &mut draft.headers);

    let can_send = outgoing_preview.can_send && !reply_blocked;
    if ui
        .add_enabled(can_send, egui::Button::new(t("subscriber.send_reply")))
        .clicked()
    {
        *pending_reply = Some(PendingReply {
            message_id: msg.id,
            reply_to,
            subject: msg.subject.clone(),
            payload_text: draft.payload.clone(),
            payload_input_format: draft.payload_input_format,
            headers: collect_headers(&draft.headers),
        });
    }
}

fn reply_status_label_key(state: &ReplyState) -> &'static str {
    match state {
        ReplyState::Replyable => "subscriber.reply_status_replyable",
        ReplyState::Sending { .. } => "subscriber.reply_status_sending",
        ReplyState::Replied => "subscriber.reply_status_replied",
        ReplyState::Failed(_) => "subscriber.reply_status_failed",
    }
}

fn render_reply_headers(ui: &mut egui::Ui, message_id: u64, headers: &mut Vec<(String, String)>) {
    egui::CollapsingHeader::new(t("subscriber.reply_headers"))
        .id_salt(("subscriber_reply_headers", message_id))
        .show(ui, |ui| {
            let mut remove_idx = None;
            for (idx, (key, value)) in headers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(key)
                            .hint_text(t("subscriber.header_key"))
                            .desired_width(120.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(value)
                            .hint_text(t("subscriber.header_value"))
                            .desired_width(200.0),
                    );
                    if ui.small_button("x").clicked() {
                        remove_idx = Some(idx);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                headers.remove(idx);
            }
            if ui.small_button(t("subscriber.add_header")).clicked() {
                headers.push((String::new(), String::new()));
            }
        });
}
