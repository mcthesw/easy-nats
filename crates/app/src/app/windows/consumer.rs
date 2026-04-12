use eframe::egui;

use crate::ui_strings as S;

use super::super::{
    editors::{AckPolicySelection, DeliverPolicySelection},
    model::EasyNatsApp,
};

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.consumer_editor.visible {
        let mut open = true;
        egui::Window::new(S::CONSUMER_CREATE)
            .open(&mut open)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                egui::Grid::new("consumer_create_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(S::CONSUMER_STREAM);
                        ui.label(&app.consumer_editor.stream_name);
                        ui.end_row();

                        ui.label(S::CONSUMER_NAME);
                        ui.text_edit_singleline(&mut app.consumer_editor.name);
                        ui.end_row();

                        ui.label(S::CONSUMER_DURABLE_MODE);
                        ui.checkbox(
                            &mut app.consumer_editor.durable,
                            S::CONSUMER_DURABLE_CHECKBOX,
                        );
                        ui.end_row();

                        ui.label(S::CONSUMER_DELIVER_POLICY);
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

                        ui.label(S::CONSUMER_ACK_POLICY);
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

                        ui.label(S::CONSUMER_FILTER_SUBJECT);
                        ui.text_edit_singleline(&mut app.consumer_editor.filter_subject);
                        ui.end_row();

                        ui.label(S::CONSUMER_MAX_DELIVER);
                        ui.text_edit_singleline(&mut app.consumer_editor.max_deliver);
                        ui.end_row();

                        ui.label(S::CONSUMER_MAX_ACK_PENDING);
                        ui.text_edit_singleline(&mut app.consumer_editor.max_ack_pending);
                        ui.end_row();

                        ui.label(S::CONSUMER_DESCRIPTION);
                        ui.text_edit_singleline(&mut app.consumer_editor.description);
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid = !app.consumer_editor.name.trim().is_empty();
                    if ui.add_enabled(valid, egui::Button::new(S::SAVE)).clicked() {
                        save_requested = true;
                    }
                    if ui.button(S::CANCEL).clicked() {
                        app.consumer_editor.visible = false;
                    }
                });
            });
        if !open {
            app.consumer_editor.visible = false;
        }
    }
    if save_requested {
        app.save_consumer_editor();
    }
}
