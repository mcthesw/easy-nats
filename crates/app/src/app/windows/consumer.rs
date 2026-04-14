use eframe::egui;

use crate::i18n::t;

use super::super::{
    editors::{AckPolicySelection, DeliverPolicySelection},
    model::EasyNatsApp,
};

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.consumer_editor.visible {
        egui::Window::new(t("consumer.create"))
            .resizable(false)
            .show(ui.ctx(), |ui| {
                egui::Grid::new("consumer_create_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(t("consumer.stream"));
                        ui.label(&app.consumer_editor.stream_name);
                        ui.end_row();

                        ui.label(t("consumer.name"));
                        ui.text_edit_singleline(&mut app.consumer_editor.name);
                        ui.end_row();

                        ui.label(t("consumer.durable_mode"));
                        ui.checkbox(
                            &mut app.consumer_editor.durable,
                            t("consumer.durable_checkbox"),
                        );
                        ui.end_row();

                        ui.label(t("consumer.deliver_policy"));
                        egui::ComboBox::from_id_salt("consumer_deliver_policy")
                            .selected_text(app.consumer_editor.deliver_policy.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.consumer_editor.deliver_policy,
                                    DeliverPolicySelection::All,
                                    DeliverPolicySelection::All.label(),
                                );
                                ui.selectable_value(
                                    &mut app.consumer_editor.deliver_policy,
                                    DeliverPolicySelection::Last,
                                    DeliverPolicySelection::Last.label(),
                                );
                                ui.selectable_value(
                                    &mut app.consumer_editor.deliver_policy,
                                    DeliverPolicySelection::New,
                                    DeliverPolicySelection::New.label(),
                                );
                            });
                        ui.end_row();

                        ui.label(t("consumer.ack_policy"));
                        egui::ComboBox::from_id_salt("consumer_ack_policy")
                            .selected_text(app.consumer_editor.ack_policy.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.consumer_editor.ack_policy,
                                    AckPolicySelection::Explicit,
                                    AckPolicySelection::Explicit.label(),
                                );
                                ui.selectable_value(
                                    &mut app.consumer_editor.ack_policy,
                                    AckPolicySelection::All,
                                    AckPolicySelection::All.label(),
                                );
                                ui.selectable_value(
                                    &mut app.consumer_editor.ack_policy,
                                    AckPolicySelection::None,
                                    AckPolicySelection::None.label(),
                                );
                            });
                        ui.end_row();

                        ui.label(t("consumer.filter_subject"));
                        ui.text_edit_singleline(&mut app.consumer_editor.filter_subject);
                        ui.end_row();

                        ui.label(t("consumer.max_deliver"));
                        ui.text_edit_singleline(&mut app.consumer_editor.max_deliver);
                        ui.end_row();

                        ui.label(t("consumer.max_ack_pending"));
                        ui.text_edit_singleline(&mut app.consumer_editor.max_ack_pending);
                        ui.end_row();

                        ui.label(t("consumer.description"));
                        ui.text_edit_singleline(&mut app.consumer_editor.description);
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.consumer_editor.name.trim().is_empty();
                    if ui
                        .add_enabled(valid, egui::Button::new(t("common.save")))
                        .clicked()
                    {
                        save_requested = true;
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.consumer_editor.visible = false;
                    }
                });
            });
    }
    if save_requested {
        app.save_consumer_editor();
    }
}
