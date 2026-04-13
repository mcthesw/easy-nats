use eframe::egui;

use crate::i18n::t;

use super::super::{editors::StorageSelection, model::EasyNatsApp};

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    render_bucket_delete_confirmation(app, ui);
    render_bucket_editor(app, ui);
}

fn render_bucket_delete_confirmation(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut do_delete = None;
    if let Some((connection_id, bucket_name)) = app.obj_store_bucket_delete_confirm.clone() {
        let mut still_open = true;
        egui::Window::new(t("obj_store.delete_bucket_confirm_title"))
            .open(&mut still_open)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(format!(
                    "{} \"{}\"?",
                    t("obj_store.delete_bucket_confirm_prompt"),
                    bucket_name
                ));
                ui.horizontal(|ui| {
                    if ui.button(t("common.delete")).clicked() {
                        do_delete = Some((connection_id, bucket_name.clone()));
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.obj_store_bucket_delete_confirm = None;
                    }
                });
            });
        if !still_open {
            app.obj_store_bucket_delete_confirm = None;
        }
    }
    if let Some((connection_id, bucket_name)) = do_delete {
        app.backend
            .send(nats_backend::BackendCommand::DeleteObjectStoreBucket {
                connection_id,
                bucket: bucket_name,
            });
        app.obj_store_bucket_delete_confirm = None;
    }
}

fn render_bucket_editor(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.obj_store_bucket_editor.visible {
        let mut open = true;
        egui::Window::new(t("obj_store.create_bucket"))
            .open(&mut open)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                egui::Grid::new("objstore_bucket_create_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("obj_store.bucket_name"));
                        ui.text_edit_singleline(&mut app.obj_store_bucket_editor.bucket);
                        ui.end_row();

                        ui.label(t("obj_store.max_bytes"));
                        ui.text_edit_singleline(&mut app.obj_store_bucket_editor.max_bytes);
                        ui.end_row();

                        ui.label(t("common.storage_label"));
                        egui::ComboBox::from_id_salt("objstore_bucket_storage")
                            .selected_text(app.obj_store_bucket_editor.storage.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.obj_store_bucket_editor.storage,
                                    StorageSelection::File,
                                    "File",
                                );
                                ui.selectable_value(
                                    &mut app.obj_store_bucket_editor.storage,
                                    StorageSelection::Memory,
                                    "Memory",
                                );
                            });
                        ui.end_row();

                        ui.label(t("common.replicas"));
                        ui.text_edit_singleline(&mut app.obj_store_bucket_editor.num_replicas);
                        ui.end_row();

                        ui.label(t("obj_store.description"));
                        ui.text_edit_singleline(&mut app.obj_store_bucket_editor.description);
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.obj_store_bucket_editor.bucket.trim().is_empty();
                    if ui
                        .add_enabled(valid, egui::Button::new(t("common.save")))
                        .clicked()
                    {
                        save_requested = true;
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.obj_store_bucket_editor.visible = false;
                    }
                });
            });
        if !open {
            app.obj_store_bucket_editor.visible = false;
        }
    }
    if save_requested {
        save_obj_store_bucket(app);
    }
}

fn save_obj_store_bucket(app: &mut EasyNatsApp) {
    let bucket = app.obj_store_bucket_editor.bucket.trim().to_string();
    if bucket.is_empty() {
        return;
    }
    let storage = match app.obj_store_bucket_editor.storage {
        StorageSelection::File => "file",
        StorageSelection::Memory => "memory",
    };

    let mut config = serde_json::json!({
        "bucket": bucket,
        "storage": storage,
    });

    if let Ok(v) = app.obj_store_bucket_editor.max_bytes.trim().parse::<i64>() {
        config["max_bytes"] = serde_json::json!(v);
    }
    if let Ok(v) = app
        .obj_store_bucket_editor
        .num_replicas
        .trim()
        .parse::<usize>()
    {
        config["num_replicas"] = serde_json::json!(v);
    }
    if !app.obj_store_bucket_editor.description.trim().is_empty() {
        config["description"] = serde_json::json!(app.obj_store_bucket_editor.description.trim());
    }
    let connection_id = app.obj_store_bucket_editor.connection_id;

    app.backend
        .send(nats_backend::BackendCommand::CreateObjectStoreBucket {
            connection_id,
            config,
        });
    app.obj_store_bucket_editor.visible = false;
}
