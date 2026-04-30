use eframe::egui;
use nats_backend::{
    BackendCommand, BackendHandle, JetStreamAccountInfoSnapshot, ServerInfoSnapshot,
};

use crate::i18n::t;

use super::common::format_bytes;
use super::types::ServerInfoState;

pub fn server_info_ui(
    ui: &mut egui::Ui,
    connection_id: u64,
    state: &mut ServerInfoState,
    backend: &BackendHandle,
) {
    ui.horizontal(|ui| {
        ui.heading(t("server_info.title"));
        if ui
            .add_enabled(!state.loading, egui::Button::new("↻"))
            .on_hover_text(t("server_info.refresh"))
            .clicked()
        {
            state.loading = true;
            backend.send(BackendCommand::GetServerInfo { connection_id });
            backend.send(BackendCommand::GetJetStreamAccountInfo { connection_id });
        }
        if state.loading {
            ui.spinner();
        }
    });
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(info) = &state.server_info {
            render_server_info(ui, info);
        }

        if let Some(account) = &state.account_info {
            ui.add_space(12.0);
            render_account_info(ui, account);
        }

        if state.server_info.is_none() && !state.loading {
            ui.weak(t("server_info.not_loaded"));
        }
    });
}

fn render_server_info(ui: &mut egui::Ui, info: &ServerInfoSnapshot) {
    egui::CollapsingHeader::new(t("server_info.section_server"))
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("server_info_grid")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    row_text(ui, t("server_info.server_name"), &info.server_name);
                    row_text(ui, t("server_info.server_id"), &info.server_id);
                    row_text(ui, t("server_info.version"), &info.version);
                    row_text(
                        ui,
                        t("server_info.host_port"),
                        &format!("{}:{}", info.host, info.port),
                    );
                    row_text(ui, t("server_info.go_version"), &info.go);
                    row_text(ui, t("server_info.proto"), &info.proto.to_string());
                    row_text(ui, t("server_info.client_id"), &info.client_id.to_string());
                    row_text(
                        ui,
                        t("server_info.max_payload"),
                        &format_bytes(info.max_payload as u64),
                    );
                    row_bool(ui, t("server_info.auth_required"), info.auth_required);
                    row_bool(ui, t("server_info.tls_required"), info.tls_required);
                });

            if !info.connect_urls.is_empty() {
                ui.add_space(8.0);
                ui.label(t("server_info.connect_urls"));
                for url in &info.connect_urls {
                    ui.monospace(format!("  • {url}"));
                }
            }
        });
}

fn render_account_info(ui: &mut egui::Ui, account: &JetStreamAccountInfoSnapshot) {
    egui::CollapsingHeader::new(t("server_info.section_jetstream"))
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("account_info_grid")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    if let Some(domain) = account.domain.as_deref() {
                        ui.label(t("server_info.domain"));
                        ui.label(domain);
                        ui.end_row();
                    }
                    row_text(
                        ui,
                        t("server_info.memory_used"),
                        &format_bytes(account.memory),
                    );
                    row_text(
                        ui,
                        t("server_info.storage_used"),
                        &format_bytes(account.storage),
                    );
                    row_text(ui, t("server_info.streams"), &account.streams.to_string());
                    row_text(
                        ui,
                        t("server_info.consumers"),
                        &account.consumers.to_string(),
                    );
                    row_text(
                        ui,
                        t("server_info.api_total"),
                        &account.api_total.to_string(),
                    );
                    row_text(
                        ui,
                        t("server_info.api_errors"),
                        &account.api_errors.to_string(),
                    );
                });

            ui.add_space(8.0);
            egui::CollapsingHeader::new(t("server_info.section_limits"))
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("limits_grid")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            limit_bytes_row(
                                ui,
                                t("server_info.max_memory"),
                                account.limits.max_memory,
                            );
                            limit_bytes_row(
                                ui,
                                t("server_info.max_storage"),
                                account.limits.max_storage,
                            );
                            limit_row(ui, t("server_info.max_streams"), account.limits.max_streams);
                            limit_row(
                                ui,
                                t("server_info.max_consumers"),
                                account.limits.max_consumers,
                            );
                        });
                });
        });
}

fn row_text(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(label);
    ui.label(value);
    ui.end_row();
}

fn row_bool(ui: &mut egui::Ui, label: &str, value: bool) {
    ui.label(label);
    ui.label(if value { "✅" } else { "❌" });
    ui.end_row();
}

fn limit_row(ui: &mut egui::Ui, label: &str, value: Option<i64>) {
    ui.label(label);
    match value {
        Some(v) if v < 0 => ui.label("∞"),
        Some(v) => ui.label(v.to_string()),
        None => ui.label("-"),
    };
    ui.end_row();
}

fn limit_bytes_row(ui: &mut egui::Ui, label: &str, value: Option<i64>) {
    ui.label(label);
    match value {
        Some(v) if v < 0 => ui.label("∞"),
        Some(v) => ui.label(format_bytes(v as u64)),
        None => ui.label("-"),
    };
    ui.end_row();
}
