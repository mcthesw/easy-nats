use eframe::egui;

use crate::i18n::t;

use super::super::{
    editors::{AuthKindSelection, ConnectionEditor},
    model::EasyNatsApp,
};

pub(crate) fn render(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    render_connection_editor(app, ui);
    render_delete_confirmation(app, ui);
}

fn render_connection_editor(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut save_requested = false;
    if app.editor.visible {
        let title = if app.editor.editing_id.is_some() {
            t("connection.connection_edit")
        } else {
            t("sidebar.connection_new")
        };
        let mut open = true;
        egui::Window::new(title)
            .open(&mut open)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                render_editor_grid(&mut app.editor, ui);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let valid =
                        !app.editor.name.trim().is_empty() && !app.editor.url.trim().is_empty();
                    if ui.add_enabled(valid, egui::Button::new(t("common.save"))).clicked() {
                        save_requested = true;
                    }
                    if ui.button(t("common.cancel")).clicked() {
                        app.editor.visible = false;
                    }
                });
            });
        if !open {
            app.editor.visible = false;
        }
    }
    if save_requested {
        app.save_editor();
    }
}

fn render_editor_grid(editor: &mut ConnectionEditor, ui: &mut egui::Ui) {
    egui::Grid::new("conn_editor_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("connection.field_name"));
            ui.text_edit_singleline(&mut editor.name);
            ui.end_row();

            ui.label(t("connection.field_url"));
            ui.text_edit_singleline(&mut editor.url);
            ui.end_row();

            ui.label(t("connection.field_auth"));
            egui::ComboBox::from_id_salt("auth_kind")
                .selected_text(editor.auth_kind.label())
                .show_ui(ui, |ui| {
                    for kind in AuthKindSelection::ALL {
                        ui.selectable_value(&mut editor.auth_kind, kind, kind.label());
                    }
                });
            ui.end_row();

            render_auth_fields(editor, ui);

            ui.label(t("connection.field_tls"));
            ui.checkbox(&mut editor.tls_enabled, t("connection.require_tls"));
            ui.end_row();
        });
}

fn render_auth_fields(editor: &mut ConnectionEditor, ui: &mut egui::Ui) {
    match editor.auth_kind {
        AuthKindSelection::None => {}
        AuthKindSelection::Token => {
            ui.label(t("connection.field_token"));
            ui.text_edit_singleline(&mut editor.token);
            ui.end_row();
        }
        AuthKindSelection::UserPassword => {
            ui.label(t("connection.field_username"));
            ui.text_edit_singleline(&mut editor.username);
            ui.end_row();
            ui.label(t("connection.field_password"));
            ui.add(egui::TextEdit::singleline(&mut editor.password).password(true));
            ui.end_row();
        }
        AuthKindSelection::NKey => {
            ui.label(t("connection.field_nkey_seed"));
            ui.add(egui::TextEdit::singleline(&mut editor.nkey_seed).password(true));
            ui.end_row();
        }
        AuthKindSelection::CredentialsFile => {
            ui.label(t("connection.field_creds_file"));
            ui.text_edit_singleline(&mut editor.creds_path);
            ui.end_row();
        }
        AuthKindSelection::TlsClientCert => {
            ui.label(t("connection.field_cert_path"));
            ui.text_edit_singleline(&mut editor.cert_path);
            ui.end_row();
            ui.label(t("connection.field_key_path"));
            ui.text_edit_singleline(&mut editor.key_path);
            ui.end_row();
        }
    }
}

fn render_delete_confirmation(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let mut do_delete = None;
    if let Some(id) = app.editor.delete_confirm {
        let conn_name = app.conn_name(id);
        let mut still_open = true;
        egui::Window::new(t("connection.connection_delete_confirm_title"))
            .open(&mut still_open)
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
        if !still_open {
            app.editor.delete_confirm = None;
        }
    }
    if let Some(id) = do_delete {
        app.delete_connection(id);
        app.editor.delete_confirm = None;
    }
}
