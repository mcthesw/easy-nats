use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format;
use crate::i18n::t;
use crate::schema::MessageSchemaManager;

use super::common::topic_history_text_edit;
use super::types::{PublisherState, TabAction};

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
    let shortcuts = publisher_shortcuts(ui);

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
        schema_manager.prepare_outgoing(connection_id, &subject, &state.payload)
    } else {
        crate::schema::OutgoingPayload {
            payload: Vec::new(),
            status: None,
            can_send: false,
        }
    };
    ui.horizontal(|ui| {
        let publish_clicked = ui
            .add_enabled(can_send, egui::Button::new(t("publisher.publish")))
            .clicked();
        if publish_clicked || (shortcuts.publish && can_send) {
            let outgoing = schema_manager.prepare_outgoing(connection_id, &subject, &state.payload);
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

        let request_clicked = ui
            .add_enabled(
                can_send && !state.waiting,
                egui::Button::new(t("publisher.request")),
            )
            .clicked();
        if request_clicked || (shortcuts.request && can_send && !state.waiting) {
            let outgoing = schema_manager.prepare_outgoing(connection_id, &subject, &state.payload);
            if outgoing.can_send {
                actions.push(TabAction::RecordTopic {
                    topic: subject.clone(),
                });
                let timeout_ms = state.timeout_ms.parse::<u64>().unwrap_or(5000);
                backend.send(BackendCommand::Request {
                    connection_id,
                    backend_id,
                    subject: subject.clone(),
                    payload: outgoing.payload,
                    headers: collect_headers(&state.headers),
                    timeout_ms,
                });
                state.response = None;
                state.waiting = true;
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
    render_response(ui, connection_id, state, schema_manager);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PublisherShortcuts {
    publish: bool,
    request: bool,
}

fn publisher_shortcuts(ui: &egui::Ui) -> PublisherShortcuts {
    let (modifiers, enter_pressed) =
        ui.input(|input| (input.modifiers, input.key_pressed(egui::Key::Enter)));

    PublisherShortcuts {
        publish: is_publish_shortcut(modifiers, enter_pressed),
        request: is_request_shortcut(modifiers, enter_pressed),
    }
}

fn is_publish_shortcut(modifiers: egui::Modifiers, enter_pressed: bool) -> bool {
    enter_pressed && modifiers.command && !modifiers.shift
}

fn is_request_shortcut(modifiers: egui::Modifiers, enter_pressed: bool) -> bool {
    enter_pressed && modifiers.command && modifiers.shift
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

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::{is_publish_shortcut, is_request_shortcut};

    fn modifiers(command: bool, shift: bool) -> egui::Modifiers {
        egui::Modifiers {
            command,
            shift,
            ..Default::default()
        }
    }

    #[test]
    fn publish_shortcut_requires_command_enter_without_shift() {
        assert!(is_publish_shortcut(modifiers(true, false), true));
        assert!(!is_publish_shortcut(modifiers(true, true), true));
        assert!(!is_publish_shortcut(modifiers(false, false), true));
        assert!(!is_publish_shortcut(modifiers(true, false), false));
    }

    #[test]
    fn request_shortcut_requires_command_shift_enter() {
        assert!(is_request_shortcut(modifiers(true, true), true));
        assert!(!is_request_shortcut(modifiers(true, false), true));
        assert!(!is_request_shortcut(modifiers(false, true), true));
        assert!(!is_request_shortcut(modifiers(true, true), false));
    }
}

fn render_response(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut PublisherState,
    schema_manager: &MessageSchemaManager,
) {
    if state.waiting {
        ui.spinner();
        ui.label(t("publisher.waiting"));
        return;
    }

    if let Some(resp) = &state.response {
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
                if let Some(subject) = resp.subject.as_deref() {
                    format::render_payload_with_schema(
                        ui,
                        &resp.payload,
                        state.response_format,
                        "pub_proto",
                        &mut state.proto_view,
                        format::SchemaRenderContext {
                            manager: schema_manager,
                            connection_id,
                            subject,
                        },
                    );
                } else {
                    format::render_payload_with_proto(
                        ui,
                        &resp.payload,
                        state.response_format,
                        "pub_proto",
                        &mut state.proto_view,
                        schema_manager,
                    );
                }
            });
    } else {
        ui.label(t("publisher.no_response"));
    }
}

fn collect_headers(headers: &[(String, String)]) -> Option<Vec<(String, String)>> {
    let non_empty: Vec<(String, String)> = headers
        .iter()
        .filter(|(k, v)| !k.trim().is_empty() || !v.trim().is_empty())
        .cloned()
        .collect();
    if non_empty.is_empty() {
        None
    } else {
        Some(non_empty)
    }
}
