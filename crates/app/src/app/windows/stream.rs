use eframe::egui;

use crate::format;
use crate::i18n::t;
use crate::tabs::payload_input_format_selector;

use super::super::model::EasyNatsApp;

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    render_create(app, ui);
    render_publish(app, ui);
}

fn render_create(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.stream_editor.visible {
        egui::Window::new(t("stream.create_title"))
            .resizable(false)
            .show(ui.ctx(), |ui| {
                let _form = crate::keyboard::Form::connected(
                    ui,
                    "stream_1",
                    true,
                    app.stream_editor.connection_id,
                );
                egui::Grid::new("stream_create_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("stream.name"));
                        crate::keyboard::singleline(ui, &mut app.stream_editor.name);
                        ui.end_row();

                        ui.label(t("stream.subjects"));
                        crate::keyboard::singleline(ui, &mut app.stream_editor.subjects);
                        ui.end_row();

                        ui.label(t("stream.storage"));
                        crate::keyboard::combo_box(
                            ui,
                            egui::ComboBox::from_id_salt("stream_storage")
                                .selected_text(app.stream_editor.storage.label()),
                            |ui| {
                                ui.selectable_value(
                                    &mut app.stream_editor.storage,
                                    super::super::editors::StorageSelection::File,
                                    "File",
                                );
                                ui.selectable_value(
                                    &mut app.stream_editor.storage,
                                    super::super::editors::StorageSelection::Memory,
                                    "Memory",
                                );
                            },
                        );
                        ui.end_row();

                        ui.label(t("stream.retention"));
                        crate::keyboard::combo_box(
                            ui,
                            egui::ComboBox::from_id_salt("stream_retention")
                                .selected_text(app.stream_editor.retention.label()),
                            |ui| {
                                ui.selectable_value(
                                    &mut app.stream_editor.retention,
                                    super::super::editors::RetentionSelection::Limits,
                                    "Limits",
                                );
                                ui.selectable_value(
                                    &mut app.stream_editor.retention,
                                    super::super::editors::RetentionSelection::Interest,
                                    "Interest",
                                );
                                ui.selectable_value(
                                    &mut app.stream_editor.retention,
                                    super::super::editors::RetentionSelection::WorkQueue,
                                    "WorkQueue",
                                );
                            },
                        );
                        ui.end_row();

                        ui.label(t("stream.max_msgs"));
                        crate::keyboard::singleline(ui, &mut app.stream_editor.max_messages);
                        ui.end_row();

                        ui.label(t("stream.max_bytes"));
                        crate::keyboard::singleline(ui, &mut app.stream_editor.max_bytes);
                        ui.end_row();

                        ui.label(t("stream.max_age"));
                        crate::keyboard::singleline(ui, &mut app.stream_editor.max_age_secs);
                        ui.end_row();

                        ui.label(t("stream.replicas"));
                        crate::keyboard::singleline(ui, &mut app.stream_editor.num_replicas);
                        ui.end_row();

                        ui.label(t("stream.description"));
                        crate::keyboard::singleline(ui, &mut app.stream_editor.description);
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.stream_editor.name.trim().is_empty();
                    if crate::keyboard::primary_button(ui, valid, t("common.save")) {
                        save_requested = true;
                    }
                    if crate::keyboard::cancel_button(ui) {
                        app.stream_editor.visible = false;
                    }
                });
            });
    }
    if save_requested {
        app.save_stream_editor();
    }
}

fn render_publish(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut publish_requested = false;
    if app.stream_publish_editor.visible {
        let title = format!(
            "{} - {}",
            t("stream.publish_message"),
            app.stream_publish_editor.stream_name
        );
        egui::Window::new(title)
            .resizable(true)
            .show(ui.ctx(), |ui| {
                let _form = crate::keyboard::Form::connected(
                    ui,
                    "stream_2",
                    true,
                    app.stream_publish_editor.connection_id,
                );
                egui::Grid::new("stream_publish_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("stream.name"));
                        ui.label(&app.stream_publish_editor.stream_name);
                        ui.end_row();

                        ui.label(t("publisher.subject"));
                        crate::keyboard::singleline(ui, &mut app.stream_publish_editor.subject);
                        ui.end_row();
                    });

                ui.add_space(4.0);
                egui::CollapsingHeader::new(t("publisher.headers"))
                    .id_salt("stream_publish_headers")
                    .default_open(false)
                    .show(ui, |ui| {
                        let mut remove_idx = None;
                        for (idx, (key, val)) in
                            app.stream_publish_editor.headers.iter_mut().enumerate()
                        {
                            ui.horizontal(|ui| {
                                crate::keyboard::text_edit(
                                    ui,
                                    egui::TextEdit::singleline(key)
                                        .hint_text(t("publisher.header_key"))
                                        .desired_width(140.0),
                                    false,
                                );
                                crate::keyboard::text_edit(
                                    ui,
                                    egui::TextEdit::singleline(val)
                                        .hint_text(t("publisher.header_value"))
                                        .desired_width(240.0),
                                    false,
                                );
                                if ui.small_button("✕").clicked() {
                                    remove_idx = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = remove_idx {
                            app.stream_publish_editor.headers.remove(idx);
                        }
                        if ui.small_button(t("publisher.add_header")).clicked() {
                            app.stream_publish_editor
                                .headers
                                .push((String::new(), String::new()));
                        }
                    });

                ui.add_space(4.0);
                let stream_subject = app.stream_publish_editor.subject.trim().to_string();
                let outgoing_preview = if !stream_subject.is_empty() {
                    Some(app.schema_manager.prepare_outgoing_with_input_format(
                        app.stream_publish_editor.connection_id,
                        &stream_subject,
                        &app.stream_publish_editor.payload,
                        app.stream_publish_editor.payload_input_format,
                    ))
                } else {
                    None
                };
                ui.horizontal(|ui| {
                    ui.label(t("publisher.payload"));
                    ui.label(t("common.payload_input_format"));
                    payload_input_format_selector(
                        ui,
                        "stream_publish_payload_input_fmt",
                        &mut app.stream_publish_editor.payload_input_format,
                    );
                    if ui.small_button(t("publisher.format_json")).clicked()
                        && let Ok(val) = serde_json::from_str::<serde_json::Value>(
                            &app.stream_publish_editor.payload,
                        )
                        && let Ok(pretty) = serde_json::to_string_pretty(&val)
                    {
                        app.stream_publish_editor.payload = pretty;
                    }
                    render_generate_json_button(
                        ui,
                        &app.schema_manager.payload_template(
                            app.stream_publish_editor.connection_id,
                            &stream_subject,
                        ),
                        &mut app.stream_publish_editor.payload,
                    );
                });
                if let Some(status) = outgoing_preview
                    .as_ref()
                    .and_then(|outgoing| outgoing.status.as_ref())
                {
                    format::render_schema_status(ui, status);
                }
                egui::ScrollArea::vertical()
                    .id_salt("stream_publish_payload")
                    .max_height(220.0)
                    .show(ui, |ui| {
                        crate::keyboard::text_edit(
                            ui,
                            egui::TextEdit::multiline(&mut app.stream_publish_editor.payload)
                                .desired_width(f32::INFINITY)
                                .desired_rows(8)
                                .code_editor()
                                .lock_focus(false),
                            true,
                        );
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.stream_publish_editor.subject.trim().is_empty()
                        && outgoing_preview
                            .as_ref()
                            .is_none_or(|outgoing| outgoing.can_send);
                    if crate::keyboard::primary_button(ui, valid, t("publisher.publish")) {
                        publish_requested = true;
                    }
                    if crate::keyboard::cancel_button(ui) {
                        app.stream_publish_editor.visible = false;
                    }
                });
            });
    }
    if publish_requested {
        app.publish_stream_editor();
    }
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
