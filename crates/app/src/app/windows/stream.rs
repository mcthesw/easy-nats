use eframe::egui;

use crate::i18n::t;

use super::super::model::EasyNatsApp;

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.stream_editor.visible {
        let mut open = true;
        egui::Window::new(t("stream.create_title"))
            .open(&mut open)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                egui::Grid::new("stream_create_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("stream.name"));
                        ui.text_edit_singleline(&mut app.stream_editor.name);
                        ui.end_row();

                        ui.label(t("stream.subjects"));
                        ui.text_edit_singleline(&mut app.stream_editor.subjects);
                        ui.end_row();

                        ui.label(t("stream.storage"));
                        egui::ComboBox::from_id_salt("stream_storage")
                            .selected_text(app.stream_editor.storage.label())
                            .show_ui(ui, |ui| {
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
                            });
                        ui.end_row();

                        ui.label(t("stream.retention"));
                        egui::ComboBox::from_id_salt("stream_retention")
                            .selected_text(app.stream_editor.retention.label())
                            .show_ui(ui, |ui| {
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
                            });
                        ui.end_row();

                        ui.label(t("stream.max_msgs"));
                        ui.text_edit_singleline(&mut app.stream_editor.max_messages);
                        ui.end_row();

                        ui.label(t("stream.max_bytes"));
                        ui.text_edit_singleline(&mut app.stream_editor.max_bytes);
                        ui.end_row();

                        ui.label(t("stream.max_age"));
                        ui.text_edit_singleline(&mut app.stream_editor.max_age_secs);
                        ui.end_row();

                        ui.label(t("stream.replicas"));
                        ui.text_edit_singleline(&mut app.stream_editor.num_replicas);
                        ui.end_row();

                        ui.label(t("stream.description"));
                        ui.text_edit_singleline(&mut app.stream_editor.description);
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.stream_editor.name.trim().is_empty();
                    if ui.add_enabled(valid, egui::Button::new(t("common.save"))).clicked() {
                        save_requested = true;
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.stream_editor.visible = false;
                    }
                });
            });
        if !open {
            app.stream_editor.visible = false;
        }
    }
    if save_requested {
        app.save_stream_editor();
    }
}
