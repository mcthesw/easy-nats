use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::ui_strings as S;

use super::types::{StreamState, TabAction};

pub(crate) fn render_consumers(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    state: &mut StreamState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
) {
    egui::CollapsingHeader::new(S::CONSUMER_HEADING)
        .id_salt(("stream_consumers", connection_id, stream_name))
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !state.consumers_fetching,
                        egui::Button::new(S::CONSUMER_REFRESH),
                    )
                    .clicked()
                {
                    backend.send(BackendCommand::ListConsumers {
                        connection_id,
                        stream: stream_name.to_string(),
                    });
                    state.consumers_fetching = true;
                }
                if ui.button(S::CONSUMER_CREATE).clicked() {
                    actions.push(TabAction::OpenConsumerCreate {
                        connection_id,
                        stream_name: stream_name.to_string(),
                    });
                }
            });

            if state.consumers_fetching {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(S::CONSUMER_LOADING);
                });
            } else if state.consumers.is_empty() {
                ui.label(S::CONSUMER_NO_CONSUMERS);
            } else {
                for consumer in &state.consumers {
                    consumer_card(ui, connection_id, stream_name, consumer, backend);
                    ui.add_space(4.0);
                }
            }
        });
}

fn consumer_card(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    consumer: &serde_json::Value,
    backend: &BackendHandle,
) {
    let name = consumer["name"].as_str().unwrap_or("(unnamed)");
    let config = &consumer["config"];
    let consumer_type = if config["deliver_subject"].as_str().is_some() {
        S::CONSUMER_TYPE_PUSH
    } else {
        S::CONSUMER_TYPE_PULL
    };

    egui::CollapsingHeader::new(name)
        .id_salt(("consumer", connection_id, stream_name, name))
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new(format!(
                "consumer_grid_{connection_id}_{stream_name}_{name}"
            ))
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                info_row(ui, S::CONSUMER_NAME, Some(name));
                info_row(ui, S::CONSUMER_TYPE, Some(consumer_type));
                info_row(
                    ui,
                    S::CONSUMER_DURABLE,
                    config["durable_name"].as_str().filter(|s| !s.is_empty()),
                );
                info_row(
                    ui,
                    S::CONSUMER_FILTER_SUBJECT,
                    config["filter_subject"].as_str().filter(|s| !s.is_empty()),
                );
                info_row(
                    ui,
                    S::CONSUMER_DELIVER_POLICY,
                    config["deliver_policy"].as_str(),
                );
                info_row(ui, S::CONSUMER_ACK_POLICY, config["ack_policy"].as_str());
                info_num_row(ui, S::CONSUMER_MAX_DELIVER, config["max_deliver"].as_i64());
                info_num_row(
                    ui,
                    S::CONSUMER_MAX_ACK_PENDING,
                    config["max_ack_pending"].as_i64(),
                );
                info_row(
                    ui,
                    S::CONSUMER_DESCRIPTION,
                    config["description"].as_str().filter(|s| !s.is_empty()),
                );
                info_u64_row(ui, S::CONSUMER_PENDING, consumer["num_pending"].as_u64());
                info_u64_row(
                    ui,
                    S::CONSUMER_ACK_PENDING,
                    consumer["num_ack_pending"].as_u64(),
                );
                info_u64_row(ui, S::CONSUMER_WAITING, consumer["num_waiting"].as_u64());
                info_u64_row(
                    ui,
                    S::CONSUMER_REDELIVERED,
                    consumer["num_redelivered"].as_u64(),
                );
            });

            ui.add_space(4.0);
            if ui.button(S::CONSUMER_DELETE).clicked() {
                backend.send(BackendCommand::DeleteConsumer {
                    connection_id,
                    stream: stream_name.to_string(),
                    name: name.to_string(),
                });
            }
        });
}

fn info_row(ui: &mut egui::Ui, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        ui.label(label);
        ui.label(value);
        ui.end_row();
    }
}

fn info_num_row(ui: &mut egui::Ui, label: &str, value: Option<i64>) {
    if let Some(value) = value {
        ui.label(label);
        ui.label(value.to_string());
        ui.end_row();
    }
}

fn info_u64_row(ui: &mut egui::Ui, label: &str, value: Option<u64>) {
    ui.label(label);
    ui.label(value.unwrap_or(0).to_string());
    ui.end_row();
}
