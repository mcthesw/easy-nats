use eframe::egui;
use nats_backend::{BackendCommand, BackendHandle};

use crate::i18n::t;

use super::types::{StreamState, TabAction};

pub(crate) fn render_consumers(
    ui: &mut egui::Ui,
    connection_id: u64,
    stream_name: &str,
    state: &mut StreamState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
) {
    egui::CollapsingHeader::new(t("consumer.heading"))
        .id_salt(("stream_consumers", connection_id, stream_name))
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !state.consumers_fetching,
                        egui::Button::new(t("consumer.refresh")),
                    )
                    .clicked()
                {
                    backend.send(BackendCommand::ListConsumers {
                        connection_id,
                        stream: stream_name.to_string(),
                    });
                    state.consumers_fetching = true;
                }
                if ui.button(t("consumer.create")).clicked() {
                    actions.push(TabAction::OpenConsumerCreate {
                        connection_id,
                        stream_name: stream_name.to_string(),
                    });
                }
            });

            if state.consumers_fetching {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(t("consumer.loading"));
                });
            } else if state.consumers.is_empty() {
                ui.label(t("consumer.no_consumers"));
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
    let name = consumer["name"].as_str().unwrap_or(t("consumer.unnamed"));
    let config = &consumer["config"];
    let consumer_type = if config["deliver_subject"].as_str().is_some() {
        t("consumer.type_push")
    } else {
        t("consumer.type_pull")
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
                info_row(ui, t("consumer.name"), Some(name));
                info_row(ui, t("consumer.type"), Some(consumer_type));
                info_row(
                    ui,
                    t("consumer.durable"),
                    config["durable_name"].as_str().filter(|s| !s.is_empty()),
                );
                info_row(
                    ui,
                    t("consumer.filter_subject"),
                    config["filter_subject"].as_str().filter(|s| !s.is_empty()),
                );
                info_row(
                    ui,
                    t("consumer.deliver_policy"),
                    config["deliver_policy"].as_str(),
                );
                info_row(ui, t("consumer.ack_policy"), config["ack_policy"].as_str());
                info_num_row(ui, t("consumer.max_deliver"), config["max_deliver"].as_i64());
                info_num_row(
                    ui,
                    t("consumer.max_ack_pending"),
                    config["max_ack_pending"].as_i64(),
                );
                info_row(
                    ui,
                    t("consumer.description"),
                    config["description"].as_str().filter(|s| !s.is_empty()),
                );
                info_u64_row(ui, t("consumer.pending"), consumer["num_pending"].as_u64());
                info_u64_row(
                    ui,
                    t("consumer.ack_pending"),
                    consumer["num_ack_pending"].as_u64(),
                );
                info_u64_row(ui, t("consumer.waiting"), consumer["num_waiting"].as_u64());
                info_u64_row(
                    ui,
                    t("consumer.redelivered"),
                    consumer["num_redelivered"].as_u64(),
                );
            });

            ui.add_space(4.0);
            if ui.button(t("consumer.delete")).clicked() {
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
