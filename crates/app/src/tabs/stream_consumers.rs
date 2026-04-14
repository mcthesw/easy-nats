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
                let consumers_snapshot: Vec<_> = state.consumers.clone();
                for consumer in &consumers_snapshot {
                    consumer_card(
                        ui,
                        connection_id,
                        stream_name,
                        consumer,
                        state,
                        backend,
                        actions,
                    );
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
    state: &mut StreamState,
    backend: &BackendHandle,
    actions: &mut Vec<TabAction>,
) {
    let name = consumer["name"].as_str().unwrap_or(t("consumer.unnamed"));
    let config = &consumer["config"];
    let consumer_type = if config["deliver_subject"].as_str().is_some() {
        t("consumer.type_push")
    } else {
        t("consumer.type_pull")
    };

    // Stable ID scope for the entire consumer card
    ui.push_id(
        egui::Id::new(("consumer_card", connection_id, stream_name, name)),
        |ui| {
            egui::CollapsingHeader::new(name)
                .id_salt("header")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("info")
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
                            info_num_row(
                                ui,
                                t("consumer.max_deliver"),
                                config["max_deliver"].as_i64(),
                            );
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
                            info_u64_row(
                                ui,
                                t("consumer.pending"),
                                consumer["num_pending"].as_u64(),
                            );
                            info_u64_row(
                                ui,
                                t("consumer.ack_pending"),
                                consumer["num_ack_pending"].as_u64(),
                            );
                            info_u64_row(
                                ui,
                                t("consumer.waiting"),
                                consumer["num_waiting"].as_u64(),
                            );
                            info_u64_row(
                                ui,
                                t("consumer.redelivered"),
                                consumer["num_redelivered"].as_u64(),
                            );
                        });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button(t("consumer.edit")).clicked() {
                            actions.push(TabAction::OpenConsumerEdit {
                                connection_id,
                                stream_name: stream_name.to_string(),
                                consumer_json: consumer.clone(),
                            });
                        }
                        let is_fetching = state.consumer_fetching.contains(name);
                        if ui
                            .add_enabled(!is_fetching, egui::Button::new(t("consumer.fetch")))
                            .on_hover_text(t("consumer.fetch_hint"))
                            .clicked()
                        {
                            backend.send(BackendCommand::FetchConsumerMessages {
                                connection_id,
                                stream: stream_name.to_string(),
                                consumer: name.to_string(),
                                batch: 10,
                            });
                            state.consumer_fetched.remove(name);
                            state.consumer_fetching.insert(name.to_string());
                        }
                        if ui.button(t("consumer.delete")).clicked() {
                            backend.send(BackendCommand::DeleteConsumer {
                                connection_id,
                                stream: stream_name.to_string(),
                                name: name.to_string(),
                            });
                        }
                    });

                    // Show fetched messages if any
                    if let Some(msgs) = state.consumer_fetched.get(name) {
                        if !msgs.is_empty() {
                            ui.add_space(4.0);
                            ui.label(format!(
                                "{} ({})",
                                t("consumer.fetched_messages"),
                                msgs.len()
                            ));
                            egui::ScrollArea::vertical()
                                .id_salt("fetched_scroll")
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for (i, msg) in msgs.iter().enumerate() {
                                        let subj = msg["subject"].as_str().unwrap_or("?");
                                        let seq = msg["seq"].as_u64().unwrap_or(0);
                                        let time = msg["time"].as_str().unwrap_or("");
                                        let header = format!("#{seq} {subj}  {time}");
                                        egui::CollapsingHeader::new(header)
                                            .id_salt(("fetched_msg", i))
                                            .default_open(false)
                                            .show(ui, |ui| {
                                                let payload = msg["payload"].as_str().unwrap_or("");
                                                ui.add(
                                                    egui::TextEdit::multiline(
                                                        &mut payload.to_string(),
                                                    )
                                                    .desired_rows(3)
                                                    .interactive(false)
                                                    .desired_width(f32::INFINITY),
                                                );
                                            });
                                    }
                                });
                        }
                    } else if state.consumer_fetching.contains(name) {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(t("consumer.fetching"));
                        });
                    }
                });
        },
    );
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
