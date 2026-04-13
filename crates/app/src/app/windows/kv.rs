use eframe::egui;

use crate::i18n::t;

use super::super::{editors::StorageSelection, model::EasyNatsApp};

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    render_bucket_delete_confirmation(app, ui);
    render_bucket_editor(app, ui);
}

fn render_bucket_delete_confirmation(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut do_delete = None;
    if let Some((connection_id, bucket_name)) = app.kv_bucket_delete_confirm.clone() {
        let mut still_open = true;
        egui::Window::new(t("kv.delete_bucket_confirm_title"))
            .open(&mut still_open)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(format!(
                    "{} \"{}\"?",
                    t("kv.delete_bucket_confirm_prompt"),
                    bucket_name
                ));
                ui.horizontal(|ui| {
                    if ui.button(t("common.delete")).clicked() {
                        do_delete = Some((connection_id, bucket_name.clone()));
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.kv_bucket_delete_confirm = None;
                    }
                });
            });
        if !still_open {
            app.kv_bucket_delete_confirm = None;
        }
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
        let mut open = true;
        egui::Window::new(t("kv.create_bucket"))
            .open(&mut open)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                egui::Grid::new("kv_bucket_create_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("kv.bucket"));
                        ui.text_edit_singleline(&mut app.kv_bucket_editor.bucket);
                        ui.end_row();

                        ui.label(t("kv.history_depth"));
                        ui.text_edit_singleline(&mut app.kv_bucket_editor.history);
                        ui.end_row();

                        ui.label(t("kv.max_age"));
                        ui.text_edit_singleline(&mut app.kv_bucket_editor.max_age_secs);
                        ui.end_row();

                        ui.label(t("kv.max_value_size"));
                        ui.text_edit_singleline(&mut app.kv_bucket_editor.max_value_size);
                        ui.end_row();

                        ui.label(t("kv.max_bytes"));
                        ui.text_edit_singleline(&mut app.kv_bucket_editor.max_bytes);
                        ui.end_row();

                        ui.label(t("kv.storage"));
                        egui::ComboBox::from_id_salt("kv_bucket_storage")
                            .selected_text(app.kv_bucket_editor.storage.label())
                            .show_ui(ui, |ui| {
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
                            });
                        ui.end_row();

                        ui.label(t("kv.replicas"));
                        ui.text_edit_singleline(&mut app.kv_bucket_editor.num_replicas);
                        ui.end_row();

                        ui.label(t("kv.description"));
                        ui.text_edit_singleline(&mut app.kv_bucket_editor.description);
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.kv_bucket_editor.bucket.trim().is_empty();
                    if ui
                        .add_enabled(valid, egui::Button::new(t("common.save")))
                        .clicked()
                    {
                        save_requested = true;
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.kv_bucket_editor.visible = false;
                    }
                });
            });
        if !open {
            app.kv_bucket_editor.visible = false;
        }
    }
    if save_requested {
        app.save_kv_bucket_editor();
    }
}
