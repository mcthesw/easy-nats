use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format;
use crate::i18n::t;
use crate::schema::MessageSchemaManager;

use super::common::{collect_headers, payload_input_format_selector, topic_history_text_edit};
use super::types::{CurrentRequest, PublisherState, RequestStatus, TabAction};

#[allow(clippy::too_many_arguments)]
pub fn publisher_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    backend_id: u64,
    state: &mut PublisherState,
    backend: &BackendHandle,
    schema_manager: &MessageSchemaManager,
    actions: &mut Vec<TabAction>,
    topic_suggestions: &[&str],
) {
    ui.horizontal(|ui| {
        ui.label(t("publisher.subject"));
        topic_history_text_edit(
            ui,
            "publisher_topic_suggestions",
            &mut state.subject,
            &mut state.subject_suggestion_idx,
            topic_suggestions,
        );
    });

    ui.add_space(4.0);
    egui::CollapsingHeader::new(t("publisher.headers"))
        .id_salt("publisher_headers")
        .show(ui, |ui| {
            let mut remove_idx = None;
            for (idx, (key, val)) in state.headers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(key)
                            .hint_text(t("publisher.header_key"))
                            .desired_width(120.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(val)
                            .hint_text(t("publisher.header_value"))
                            .desired_width(200.0),
                    );
                    if ui.small_button("✕").clicked() {
                        remove_idx = Some(idx);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                state.headers.remove(idx);
            }
            if ui.small_button(t("publisher.add_header")).clicked() {
                state.headers.push((String::new(), String::new()));
            }
        });

    ui.add_space(4.0);
    let subject = state.subject.trim().to_owned();
    let payload_template = schema_manager.payload_template(connection_id, &subject);
    ui.horizontal(|ui| {
        ui.label(t("publisher.payload"));
        ui.label(t("common.payload_input_format"));
        payload_input_format_selector(ui, "pub_payload_input_fmt", &mut state.payload_input_format);
        if ui.small_button(t("publisher.format_json")).clicked()
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&state.payload)
            && let Ok(pretty) = serde_json::to_string_pretty(&val)
        {
            state.payload = pretty;
        }
        render_generate_json_button(ui, &payload_template, &mut state.payload);
    });
    egui::ScrollArea::vertical()
        .id_salt("publisher_payload")
        .max_height(200.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.payload)
                    .desired_width(f32::INFINITY)
                    .desired_rows(6)
                    .code_editor(),
            );
        });

    ui.add_space(4.0);
    let can_send = !subject.is_empty();
    let outgoing_preview = if can_send {
        schema_manager.prepare_outgoing_with_input_format(
            connection_id,
            &subject,
            &state.payload,
            state.payload_input_format,
        )
    } else {
        crate::schema::OutgoingPayload {
            payload: Vec::new(),
            status: None,
            can_send: false,
        }
    };
    let can_submit = can_send && outgoing_preview.can_send;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(can_submit, egui::Button::new(t("publisher.publish")))
            .clicked()
        {
            let outgoing = schema_manager.prepare_outgoing_with_input_format(
                connection_id,
                &subject,
                &state.payload,
                state.payload_input_format,
            );
            if outgoing.can_send {
                actions.push(TabAction::RecordTopic {
                    topic: subject.clone(),
                });
                backend.send(BackendCommand::Publish {
                    connection_id,
                    subject: subject.clone(),
                    payload: outgoing.payload,
                    headers: collect_headers(&state.headers),
                });
            }
        }

        ui.separator();
        ui.label(t("publisher.timeout"));
        ui.add(egui::TextEdit::singleline(&mut state.timeout_ms).desired_width(60.0));

        if ui
            .add_enabled(
                can_submit && !state.is_request_waiting(),
                egui::Button::new(t("publisher.request")),
            )
            .clicked()
        {
            let outgoing = schema_manager.prepare_outgoing_with_input_format(
                connection_id,
                &subject,
                &state.payload,
                state.payload_input_format,
            );
            if outgoing.can_send {
                actions.push(TabAction::RecordTopic {
                    topic: subject.clone(),
                });
                let timeout_ms = state.timeout_ms.parse::<u64>().unwrap_or(5000);
                let request_id = state.start_request(subject.clone());
                backend.send(BackendCommand::Request {
                    connection_id,
                    backend_id,
                    request_id,
                    subject: subject.clone(),
                    payload: outgoing.payload,
                    headers: collect_headers(&state.headers),
                    timeout_ms,
                });
            }
        }
    });
    if let Some(status) = &outgoing_preview.status {
        format::render_schema_status(ui, status);
    }

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(t("publisher.response"));
        format::format_selector(ui, "pub_resp_fmt", &mut state.response_format);
    });
    render_current_response(ui, connection_id, state, schema_manager);
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

fn render_current_response(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut PublisherState,
    schema_manager: &MessageSchemaManager,
) {
    let PublisherState {
        current_request,
        response_format,
        proto_view,
        ..
    } = state;
    let Some(request) = current_request.as_ref() else {
        ui.label(t("publisher.no_response"));
        return;
    };
    render_request_detail(
        ui,
        connection_id,
        request,
        *response_format,
        proto_view,
        schema_manager,
    );
}

fn render_request_detail(
    ui: &mut egui::Ui,
    connection_id: u64,
    request: &CurrentRequest,
    response_format: format::PayloadFormat,
    proto_view: &mut crate::proto::ProtoViewState,
    schema_manager: &MessageSchemaManager,
) {
    match request.status {
        RequestStatus::Waiting => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t("publisher.waiting"));
            });
        }
        RequestStatus::TimedOut | RequestStatus::NoResponders | RequestStatus::Failed => {
            if let Some(message) = &request.error_message {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("{}: {message}", t(request.status.label_key())),
                );
            }
        }
        RequestStatus::Responded => {}
    }

    if let Some(resp) = &request.response {
        if !resp.headers.is_empty() {
            ui.label(t("publisher.response_headers"));
            egui::Grid::new("resp_headers")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    for (k, v) in &resp.headers {
                        ui.label(k);
                        ui.label(v);
                        ui.end_row();
                    }
                });
            ui.add_space(4.0);
        }
        ui.label(t("publisher.response_payload"));
        egui::ScrollArea::vertical()
            .id_salt("resp_payload")
            .max_height(200.0)
            .show(ui, |ui| {
                let subject = resp.subject.as_deref().unwrap_or(&request.subject);
                format::render_payload_with_schema(
                    ui,
                    &resp.payload,
                    response_format,
                    "pub_proto",
                    proto_view,
                    format::SchemaRenderContext {
                        manager: schema_manager,
                        connection_id,
                        subject,
                    },
                );
            });
    }
}
