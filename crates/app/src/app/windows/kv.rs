use eframe::egui;

use crate::format;
use crate::i18n::t;
use crate::schema::kv_subject;
use crate::tabs::payload_input_format_selector;

use super::super::{editors::StorageSelection, model::EasyNatsApp};

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    render_bucket_delete_confirmation(app, ui);
    render_bucket_editor(app, ui);
    render_bucket_edit_editor(app, ui);
    render_entry_create_editor(app, ui);
}

fn render_bucket_delete_confirmation(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut do_delete = None;
    if let Some((connection_id, bucket_name)) = app.kv_bucket_delete_confirm.clone() {
        egui::Modal::new(egui::Id::new("kv.delete_bucket_confirm_title")).show(ui.ctx(), |ui| {
            ui.heading(t("kv.delete_bucket_confirm_title"));
            let _form = crate::keyboard::Form::new(ui, "kv_1", true);
            ui.label(format!(
                "{} \"{}\"?",
                t("kv.delete_bucket_confirm_prompt"),
                bucket_name
            ));
            ui.horizontal(|ui| {
                if ui.button(t("common.delete")).clicked() {
                    do_delete = Some((connection_id, bucket_name.clone()));
                }
                if crate::keyboard::cancel_button(ui) {
                    app.kv_bucket_delete_confirm = None;
                }
            });
        });
    }
    if let Some((connection_id, bucket_name)) = do_delete {
        app.backend
            .send(nats_backend::BackendCommand::DeleteKvBucket {
                connection_id,
                bucket: bucket_name,
            });
        app.kv_bucket_delete_confirm = None;
    }
}

fn render_bucket_editor(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.kv_bucket_editor.visible {
        egui::Window::new(t("kv.create_bucket"))
            .resizable(false)
            .show(ui.ctx(), |ui| {
                let _form = crate::keyboard::Form::connected(
                    ui,
                    "kv_2",
                    true,
                    app.kv_bucket_editor.connection_id,
                );
                egui::Grid::new("kv_bucket_create_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("kv.bucket"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_editor.bucket);
                        ui.end_row();

                        ui.label(t("kv.history_depth"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_editor.history);
                        ui.end_row();

                        ui.label(t("kv.max_age"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_editor.max_age_secs);
                        ui.end_row();

                        ui.label(t("kv.max_value_size"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_editor.max_value_size);
                        ui.end_row();

                        ui.label(t("kv.max_bytes"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_editor.max_bytes);
                        ui.end_row();

                        ui.label(t("kv.storage"));
                        crate::keyboard::combo_box(
                            ui,
                            egui::ComboBox::from_id_salt("kv_bucket_storage")
                                .selected_text(app.kv_bucket_editor.storage.label()),
                            |ui| {
                                ui.selectable_value(
                                    &mut app.kv_bucket_editor.storage,
                                    StorageSelection::File,
                                    "File",
                                );
                                ui.selectable_value(
                                    &mut app.kv_bucket_editor.storage,
                                    StorageSelection::Memory,
                                    "Memory",
                                );
                            },
                        );
                        ui.end_row();

                        ui.label(t("kv.replicas"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_editor.num_replicas);
                        ui.end_row();

                        ui.label(t("kv.description"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_editor.description);
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.kv_bucket_editor.bucket.trim().is_empty();
                    if crate::keyboard::primary_button(ui, valid, t("common.save")) {
                        save_requested = true;
                    }
                    if crate::keyboard::cancel_button(ui) {
                        app.kv_bucket_editor.visible = false;
                    }
                });
            });
    }
    if save_requested {
        app.save_kv_bucket_editor();
    }
}

fn render_bucket_edit_editor(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.kv_bucket_edit_editor.visible {
        egui::Window::new(t("kv.edit_bucket"))
            .resizable(false)
            .show(ui.ctx(), |ui| {
                let _form = crate::keyboard::Form::connected(
                    ui,
                    "kv_3",
                    true,
                    app.kv_bucket_edit_editor.connection_id,
                );
                egui::Grid::new("kv_bucket_edit_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("kv.bucket"));
                        ui.label(&app.kv_bucket_edit_editor.bucket);
                        ui.end_row();

                        ui.label(t("kv.history_depth"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_edit_editor.history);
                        ui.end_row();

                        ui.label(t("kv.max_age"));
                        crate::keyboard::singleline(
                            ui,
                            &mut app.kv_bucket_edit_editor.max_age_secs,
                        );
                        ui.end_row();

                        ui.label(t("kv.max_value_size"));
                        crate::keyboard::singleline(
                            ui,
                            &mut app.kv_bucket_edit_editor.max_value_size,
                        );
                        ui.end_row();

                        ui.label(t("kv.max_bytes"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_edit_editor.max_bytes);
                        ui.end_row();

                        ui.label(t("kv.replicas"));
                        crate::keyboard::singleline(
                            ui,
                            &mut app.kv_bucket_edit_editor.num_replicas,
                        );
                        ui.end_row();

                        ui.label(t("kv.description"));
                        crate::keyboard::singleline(ui, &mut app.kv_bucket_edit_editor.description);
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if crate::keyboard::primary_button(ui, true, t("common.save")) {
                        save_requested = true;
                    }
                    if crate::keyboard::cancel_button(ui) {
                        app.kv_bucket_edit_editor.visible = false;
                    }
                });
            });
    }
    if save_requested {
        app.save_kv_bucket_edit_editor();
    }
}

fn render_entry_create_editor(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.kv_entry_create_editor.visible {
        let title = format!(
            "{} - {}",
            t("kv.new_entry"),
            app.kv_entry_create_editor.bucket_name
        );
        egui::Window::new(title)
            .resizable(true)
            .show(ui.ctx(), |ui| {
                let _form = crate::keyboard::Form::connected(
                    ui,
                    "kv_4",
                    true,
                    app.kv_entry_create_editor.connection_id,
                );
                egui::Grid::new("kv_entry_create_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("kv.bucket"));
                        ui.label(&app.kv_entry_create_editor.bucket_name);
                        ui.end_row();

                        ui.label(t("kv.key"));
                        crate::keyboard::singleline(ui, &mut app.kv_entry_create_editor.key);
                        ui.end_row();
                    });

                ui.add_space(4.0);
                let entry_subject = kv_subject(
                    &app.kv_entry_create_editor.bucket_name,
                    app.kv_entry_create_editor.key.trim(),
                );
                let outgoing_preview = if !app.kv_entry_create_editor.key.trim().is_empty() {
                    Some(app.schema_manager.prepare_outgoing_with_input_format(
                        app.kv_entry_create_editor.connection_id,
                        &entry_subject,
                        &app.kv_entry_create_editor.value,
                        app.kv_entry_create_editor.value_input_format,
                    ))
                } else {
                    None
                };
                ui.horizontal(|ui| {
                    ui.label(t("kv.value_editor"));
                    ui.label(t("common.payload_input_format"));
                    payload_input_format_selector(
                        ui,
                        "kv_entry_create_value_input_fmt",
                        &mut app.kv_entry_create_editor.value_input_format,
                    );
                    if ui.small_button(t("kv.format_json")).clicked()
                        && let Ok(val) = serde_json::from_str::<serde_json::Value>(
                            &app.kv_entry_create_editor.value,
                        )
                        && let Ok(pretty) = serde_json::to_string_pretty(&val)
                    {
                        app.kv_entry_create_editor.value = pretty;
                    }
                    render_generate_json_button(
                        ui,
                        &app.schema_manager.payload_template(
                            app.kv_entry_create_editor.connection_id,
                            &entry_subject,
                        ),
                        &mut app.kv_entry_create_editor.value,
                    );
                });
                if let Some(status) = outgoing_preview
                    .as_ref()
                    .and_then(|outgoing| outgoing.status.as_ref())
                {
                    format::render_schema_status(ui, status);
                }
                egui::ScrollArea::vertical()
                    .id_salt("kv_entry_create_value")
                    .max_height(220.0)
                    .show(ui, |ui| {
                        crate::keyboard::text_edit(
                            ui,
                            egui::TextEdit::multiline(&mut app.kv_entry_create_editor.value)
                                .desired_width(f32::INFINITY)
                                .desired_rows(8)
                                .code_editor()
                                .lock_focus(false),
                            true,
                        );
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.kv_entry_create_editor.key.trim().is_empty()
                        && outgoing_preview
                            .as_ref()
                            .is_none_or(|outgoing| outgoing.can_send);
                    if crate::keyboard::primary_button(ui, valid, t("common.save")) {
                        save_requested = true;
                    }
                    if crate::keyboard::cancel_button(ui) {
                        app.kv_entry_create_editor.visible = false;
                    }
                });
            });
    }
    if save_requested {
        app.save_kv_entry_create_editor();
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
