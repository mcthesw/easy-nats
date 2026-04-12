use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::format;
use crate::ui_strings as S;

use super::types::PublisherState;

pub fn publisher_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut PublisherState,
    backend: &BackendHandle,
) {
    ui.horizontal(|ui| {
        ui.label(S::PUBLISHER_SUBJECT);
        ui.text_edit_singleline(&mut state.subject);
    });

    ui.add_space(4.0);
    egui::CollapsingHeader::new(S::PUBLISHER_HEADERS)
        .id_salt("publisher_headers")
        .show(ui, |ui| {
            let mut remove_idx = None;
            for (idx, (key, val)) in state.headers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(key)
                            .hint_text(S::PUBLISHER_HEADER_KEY)
                            .desired_width(120.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(val)
                            .hint_text(S::PUBLISHER_HEADER_VALUE)
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
            if ui.small_button(S::PUBLISHER_ADD_HEADER).clicked() {
                state.headers.push((String::new(), String::new()));
            }
        });

    ui.add_space(4.0);
    ui.label(S::PUBLISHER_PAYLOAD);
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
    ui.horizontal(|ui| {
        let can_send = !state.subject.trim().is_empty();
        if ui
            .add_enabled(can_send, egui::Button::new(S::PUBLISHER_PUBLISH))
            .clicked()
        {
            backend.send(BackendCommand::Publish {
                connection_id,
                subject: state.subject.clone(),
                payload: state.payload.as_bytes().to_vec(),
                headers: collect_headers(&state.headers),
            });
        }

        ui.separator();
        ui.label(S::PUBLISHER_TIMEOUT);
        ui.add(egui::TextEdit::singleline(&mut state.timeout_ms).desired_width(60.0));

        if ui
            .add_enabled(
                can_send && !state.waiting,
                egui::Button::new(S::PUBLISHER_REQUEST),
            )
            .clicked()
        {
            let timeout_ms = state.timeout_ms.parse::<u64>().unwrap_or(5000);
            backend.send(BackendCommand::Request {
                connection_id,
                subject: state.subject.clone(),
                payload: state.payload.as_bytes().to_vec(),
                headers: collect_headers(&state.headers),
                timeout_ms,
            });
            state.response = None;
            state.waiting = true;
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(S::PUBLISHER_RESPONSE);
        format::format_selector(ui, "pub_resp_fmt", &mut state.response_format);
    });
    render_response(ui, state);
}

fn render_response(ui: &mut egui::Ui, state: &mut PublisherState) {
    if state.waiting {
        ui.spinner();
        ui.label(S::PUBLISHER_WAITING);
        return;
    }

    if let Some(resp) = &state.response {
        if !resp.headers.is_empty() {
            ui.label(S::PUBLISHER_RESPONSE_HEADERS);
            egui::Grid::new("resp_headers")
                .num_columns(2)
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
        ui.label(S::PUBLISHER_RESPONSE_PAYLOAD);
        egui::ScrollArea::vertical()
            .id_salt("resp_payload")
            .max_height(200.0)
            .show(ui, |ui| {
                format::render_payload(ui, &resp.payload, state.response_format);
            });
    } else {
        ui.label(S::PUBLISHER_NO_RESPONSE);
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
