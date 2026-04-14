use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

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

fn render_server_info(ui: &mut egui::Ui, info: &serde_json::Value) {
    egui::CollapsingHeader::new(t("server_info.section_server"))
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("server_info_grid")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    row(ui, t("server_info.server_name"), &info["server_name"]);
                    row(ui, t("server_info.server_id"), &info["server_id"]);
                    row(ui, t("server_info.version"), &info["version"]);
                    row(
                        ui,
                        t("server_info.host_port"),
                        &serde_json::json!(format!(
                            "{}:{}",
                            info["host"].as_str().unwrap_or(""),
                            info["port"].as_u64().unwrap_or(0)
                        )),
                    );
                    row(ui, t("server_info.go_version"), &info["go"]);
                    row(ui, t("server_info.proto"), &info["proto"]);
                    row(ui, t("server_info.client_id"), &info["client_id"]);
                    if let Some(max) = info["max_payload"].as_u64() {
                        ui.label(t("server_info.max_payload"));
                        ui.label(format_bytes(max));
                        ui.end_row();
                    }
                    row_bool(ui, t("server_info.auth_required"), &info["auth_required"]);
                    row_bool(ui, t("server_info.tls_required"), &info["tls_required"]);
                });

            if let Some(urls) = info["connect_urls"].as_array()
                && !urls.is_empty()
            {
                ui.add_space(8.0);
                ui.label(t("server_info.connect_urls"));
                for url in urls {
                    if let Some(u) = url.as_str() {
                        ui.monospace(format!("  • {u}"));
                    }
                }
            }
        });
}

fn render_account_info(ui: &mut egui::Ui, account: &serde_json::Value) {
    egui::CollapsingHeader::new(t("server_info.section_jetstream"))
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("account_info_grid")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    if let Some(domain) = account["domain"].as_str() {
                        ui.label(t("server_info.domain"));
                        ui.label(domain);
                        ui.end_row();
                    }
                    if let Some(mem) = account["memory"].as_u64() {
                        ui.label(t("server_info.memory_used"));
                        ui.label(format_bytes(mem));
                        ui.end_row();
                    }
                    if let Some(storage) = account["storage"].as_u64() {
                        ui.label(t("server_info.storage_used"));
                        ui.label(format_bytes(storage));
                        ui.end_row();
                    }
                    row(ui, t("server_info.streams"), &account["streams"]);
                    row(ui, t("server_info.consumers"), &account["consumers"]);
                    row(ui, t("server_info.api_total"), &account["api_total"]);
                    row(ui, t("server_info.api_errors"), &account["api_errors"]);
                });

            if let Some(limits) = account.get("limits") {
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
                                    limits["max_memory"].as_i64(),
                                );
                                limit_bytes_row(
                                    ui,
                                    t("server_info.max_storage"),
                                    limits["max_storage"].as_i64(),
                                );
                                limit_row(
                                    ui,
                                    t("server_info.max_streams"),
                                    limits["max_streams"].as_i64(),
                                );
                                limit_row(
                                    ui,
                                    t("server_info.max_consumers"),
                                    limits["max_consumers"].as_i64(),
                                );
                            });
                    });
            }
        });
}

fn row(ui: &mut egui::Ui, label: &str, value: &serde_json::Value) {
    ui.label(label);
    match value {
        serde_json::Value::String(s) => {
            ui.label(s.as_str());
        }
        serde_json::Value::Number(n) => {
            ui.label(n.to_string());
        }
        _ => {
            ui.label(value.to_string());
        }
    }
    ui.end_row();
}

fn row_bool(ui: &mut egui::Ui, label: &str, value: &serde_json::Value) {
    ui.label(label);
    let b = value.as_bool().unwrap_or(false);
    ui.label(if b { "✅" } else { "❌" });
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
