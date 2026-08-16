use eframe::egui;

use crate::i18n::t;

use super::super::{
    editors::{AuthKindSelection, ConnectionEditor, ConnectionTestState},
    model::EasyNatsApp,
};

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    render_connection_editor(app, ui);
    render_delete_confirmation(app, ui);
}

fn render_connection_editor(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    let mut test_requested = false;
    if app.editor.visible {
        let title = if app.editor.editing_id.is_some() {
            t("connection.connection_edit")
        } else {
            t("sidebar.connection_new")
        };
        egui::Window::new(title)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                if render_editor_grid(&mut app.editor, ui) {
                    app.editor.invalidate_test();
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid =
                        !app.editor.name.trim().is_empty() && !app.editor.url.trim().is_empty();
                    if ui
                        .add_enabled(valid, egui::Button::new(t("common.save")))
                        .clicked()
                    {
                        save_requested = true;
                    }
                    let testing =
                        matches!(app.editor.test_state, ConnectionTestState::Pending { .. });
                    if ui
                        .add_enabled(valid && !testing, egui::Button::new(t("connection.test")))
                        .clicked()
                    {
                        test_requested = true;
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.editor.invalidate_test();
                        app.editor.visible = false;
                    }
                });
                // A status line is always rendered so a result never resizes
                // the window; long errors truncate, full text on hover.
                ui.horizontal(|ui| match &app.editor.test_state {
                    ConnectionTestState::Idle => {
                        // Blank-but-full-height placeholder: an empty string
                        // layouts shorter than a real line and would let the
                        // window shrink a few px.
                        ui.label(" ");
                    }
                    ConnectionTestState::Pending { .. } => {
                        ui.spinner();
                    }
                    ConnectionTestState::Succeeded => {
                        // Muted green tuned for the active theme: pure GREEN is
                        // unreadable on light backgrounds.
                        let success = if ui.visuals().dark_mode {
                            egui::Color32::from_rgb(129, 199, 132)
                        } else {
                            egui::Color32::from_rgb(46, 125, 50)
                        };
                        ui.colored_label(success, t("connection.test_success"));
                    }
                    ConnectionTestState::Failed(message) => {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!(
                                    "{}: {}",
                                    t("connection.test_failed"),
                                    message
                                ))
                                .color(ui.visuals().error_fg_color),
                            )
                            .truncate(),
                        )
                        .on_hover_text(message);
                    }
                });
            });
    }
    if save_requested {
        app.save_editor();
    }
    if test_requested {
        app.test_editor_connection();
    }
}

fn render_editor_grid(editor: &mut ConnectionEditor, ui: &mut egui::Ui) -> bool {
    let mut connection_changed = false;
    egui::Grid::new("conn_editor_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("connection.field_name"));
            ui.text_edit_singleline(&mut editor.name);
            ui.end_row();

            ui.label(t("connection.field_url"));
            connection_changed |= ui.text_edit_singleline(&mut editor.url).changed();
            ui.end_row();

            ui.label(t("connection.field_metrics_endpoint"));
            ui.add(
                egui::TextEdit::singleline(&mut editor.metrics_endpoint)
                    .hint_text(t("connection.metrics_endpoint_hint")),
            );
            ui.end_row();

            ui.label(t("connection.field_auth"));
            let previous_auth_kind = editor.auth_kind;
            egui::ComboBox::from_id_salt("auth_kind")
                .selected_text(editor.auth_kind.label())
                .show_ui(ui, |ui| {
                    for kind in AuthKindSelection::ALL {
                        ui.selectable_value(&mut editor.auth_kind, kind, kind.label());
                    }
                });
            connection_changed |= editor.auth_kind != previous_auth_kind;
            ui.end_row();

            connection_changed |= render_auth_fields(editor, ui);

            ui.label(t("connection.field_tls_mode"));
            let previous_tls_mode = (editor.tls_enabled, editor.tls_first);
            let mut mode = if editor.tls_first {
                2
            } else if editor.tls_enabled {
                1
            } else {
                0
            };
            egui::ComboBox::from_id_salt("tls_mode")
                .selected_text(match mode {
                    1 => t("connection.tls_mode_required"),
                    2 => t("connection.tls_mode_first"),
                    _ => t("connection.tls_mode_off"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut mode, 0, t("connection.tls_mode_off"));
                    ui.selectable_value(&mut mode, 1, t("connection.tls_mode_required"));
                    ui.selectable_value(&mut mode, 2, t("connection.tls_mode_first"));
                });
            editor.tls_enabled = mode != 0;
            editor.tls_first = mode == 2;
            connection_changed |= (editor.tls_enabled, editor.tls_first) != previous_tls_mode;
            ui.end_row();
        });
    connection_changed
}

fn render_auth_fields(editor: &mut ConnectionEditor, ui: &mut egui::Ui) -> bool {
    match editor.auth_kind {
        AuthKindSelection::None => false,
        AuthKindSelection::Token => {
            ui.label(t("connection.field_token"));
            let changed = ui
                .add(egui::TextEdit::singleline(&mut editor.token).password(true))
                .changed();
            ui.end_row();
            changed
        }
        AuthKindSelection::UserPassword => {
            ui.label(t("connection.field_username"));
            let mut changed = ui.text_edit_singleline(&mut editor.username).changed();
            ui.end_row();
            ui.label(t("connection.field_password"));
            changed |= ui
                .add(egui::TextEdit::singleline(&mut editor.password).password(true))
                .changed();
            ui.end_row();
            changed
        }
        AuthKindSelection::NKey => {
            ui.label(t("connection.field_nkey_seed"));
            let changed = ui
                .add(egui::TextEdit::singleline(&mut editor.nkey_seed).password(true))
                .changed();
            ui.end_row();
            changed
        }
        AuthKindSelection::CredentialsFile => {
            ui.label(t("connection.field_creds_file"));
            let changed = ui.text_edit_singleline(&mut editor.creds_path).changed();
            ui.end_row();
            changed
        }
        AuthKindSelection::TlsClientCert => {
            ui.label(t("connection.field_cert_path"));
            let mut changed = ui.text_edit_singleline(&mut editor.cert_path).changed();
            ui.end_row();
            ui.label(t("connection.field_key_path"));
            changed |= ui.text_edit_singleline(&mut editor.key_path).changed();
            ui.end_row();
            changed
        }
    }
}

fn render_delete_confirmation(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut do_delete = None;
    if let Some(id) = app.editor.delete_confirm {
        let conn_name = app.conn_name(id);
        egui::Window::new(t("connection.connection_delete_confirm_title"))
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(format!(
                    "{} \"{}\"?",
                    t("connection.connection_delete_prompt"),
                    conn_name
                ));
                ui.horizontal(|ui| {
                    if ui.button(t("common.delete")).clicked() {
                        do_delete = Some(id);
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.editor.delete_confirm = None;
                    }
                });
            });
    }
    if let Some(id) = do_delete {
        app.delete_connection(id);
        app.editor.delete_confirm = None;
    }
}
