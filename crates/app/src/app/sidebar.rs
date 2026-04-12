use eframe::egui;
use nats_backend::{BackendCommand, ConnectionStatusKind};

use crate::i18n::{self, t, Language};
use crate::tabs::{KvBucketState, PublisherState, StreamState, SubscriberState, TabKind};

use super::{
    editors::{KvBucketCreateEditor, StreamCreateEditor},
    model::EasyNatsApp,
};

pub(crate) enum SidebarAction {
    Select(u64),
    Connect(u64),
    Disconnect(u64),
    OpenTab(Box<TabKind>),
    OpenStreamCreate(u64),
    OpenKvBucketCreate(u64),
}

pub(crate) fn render_sidebar(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    egui::Panel::left("sidebar_panel")
        .default_size(220.0)
        .show_inside(ui, |ui| {
            render_sidebar_header(app, ui);
            ui.separator();
            render_connection_tree(app, ui);
            render_selected_connection_actions(app, ui);
        });
}

fn render_sidebar_header(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading(t("sidebar.connections_heading"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let icon = if app.dark_mode {
                t("common.theme_light")
            } else {
                t("common.theme_dark")
            };
            if ui.small_button(icon).clicked() {
                app.dark_mode = !app.dark_mode;
                if app.dark_mode {
                    ui.ctx().set_visuals(egui::Visuals::dark());
                } else {
                    ui.ctx().set_visuals(egui::Visuals::light());
                }
                app.settings.dark_mode = app.dark_mode;
                app.settings.save();
            }

            // Language selector
            let mut lang = i18n::current_language();
            egui::ComboBox::from_id_salt("lang_selector")
                .width(60.0)
                .selected_text(lang.label())
                .show_ui(ui, |ui| {
                    for l in Language::ALL {
                        if ui.selectable_value(&mut lang, l, l.label()).changed() {
                            i18n::set_language(lang);
                            app.settings.language = lang;
                            app.settings.save();
                        }
                    }
                });

            if ui
                .small_button("＋")
                .on_hover_text(t("sidebar.connection_new"))
                .clicked()
            {
                app.open_new_editor();
            }
        });
    });
}

fn render_connection_tree(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    let conn_data: Vec<(u64, String, ConnectionStatusKind)> = app
        .config
        .connections
        .iter()
        .map(|c| {
            let status = app
                .conn_statuses
                .get(&c.id)
                .cloned()
                .unwrap_or(ConnectionStatusKind::Disconnected);
            (c.id, c.name.clone(), status)
        })
        .collect();

    let mut action = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (id, name, status) in &conn_data {
            render_connection_row(app, ui, *id, name, status, &mut action);
            if matches!(status, ConnectionStatusKind::Connected) {
                render_resource_tree(app, ui, *id, name, &mut action);
            }
        }
    });

    apply_sidebar_action(app, action);
}

fn render_connection_row(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    name: &str,
    status: &ConnectionStatusKind,
    action: &mut Option<SidebarAction>,
) {
    let selected = app.selected_conn == Some(id);
    let status_color = match status {
        ConnectionStatusKind::Connected => egui::Color32::GREEN,
        ConnectionStatusKind::Connecting => egui::Color32::YELLOW,
        ConnectionStatusKind::Disconnected => egui::Color32::GRAY,
        ConnectionStatusKind::Error(_) => egui::Color32::RED,
    };

    ui.horizontal(|ui| {
        ui.colored_label(status_color, "●");
        if ui
            .selectable_label(selected, name)
            .clicked()
        {
            *action = Some(SidebarAction::Select(id));
        }

        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| match status {
                ConnectionStatusKind::Connected => {
                    if ui.small_button("⏏").on_hover_text(t("connection.disconnect")).clicked() {
                        *action = Some(SidebarAction::Disconnect(id));
                    }
                }
                ConnectionStatusKind::Connecting => {}
                _ => {
                    if ui.small_button("▶").on_hover_text(t("connection.connect")).clicked() {
                        *action = Some(SidebarAction::Connect(id));
                    }
                }
            },
        );
    });
}

fn render_resource_tree(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    name: &str,
    action: &mut Option<SidebarAction>,
) {
    ui.indent(format!("tree_{id}"), |ui| {
        render_pubsub_section(app, ui, id, name, action);
        render_streams_section(app, ui, id, name, action);
        render_kv_section(app, ui, id, name, action);
        egui::CollapsingHeader::new(t("sidebar.section_object_store"))
            .id_salt(format!("objstore_{id}"))
            .show(ui, |ui| {
                ui.weak(t("common.object_store_wip"));
            });
    });
}

fn render_pubsub_section(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    name: &str,
    action: &mut Option<SidebarAction>,
) {
    egui::CollapsingHeader::new(t("sidebar.section_pubsub"))
        .id_salt(format!("pubsub_{id}"))
        .show(ui, |ui| {
            if ui.selectable_label(false, t("sidebar.open_publisher")).clicked() {
                let instance_id = app.next_tab_instance;
                app.next_tab_instance += 1;
                *action = Some(SidebarAction::OpenTab(Box::new(TabKind::Publisher {
                    connection_id: id,
                    connection_name: name.to_string(),
                    instance_id,
                    state: PublisherState::default(),
                })));
            }
            if ui.selectable_label(false, t("sidebar.open_subscriber")).clicked() {
                let instance_id = app.next_tab_instance;
                app.next_tab_instance += 1;
                *action = Some(SidebarAction::OpenTab(Box::new(TabKind::Subscriber {
                    connection_id: id,
                    connection_name: name.to_string(),
                    instance_id,
                    state: SubscriberState::default(),
                })));
            }
        });
}

fn render_streams_section(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    name: &str,
    action: &mut Option<SidebarAction>,
) {
    egui::CollapsingHeader::new(t("sidebar.section_streams"))
        .id_salt(format!("streams_{id}"))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .small_button("＋")
                    .on_hover_text(t("stream.create_title"))
                    .clicked()
                {
                    *action = Some(SidebarAction::OpenStreamCreate(id));
                }
                if ui
                    .small_button("↻")
                    .on_hover_text(t("stream.refresh"))
                    .clicked()
                {
                    app.backend
                        .send(BackendCommand::ListStreams { connection_id: id });
                }
            });
            if let Some(infos) = app.stream_lists.get(&id) {
                for info in infos {
                    if let Some(stream_name) = info["config"]["name"].as_str()
                        && ui.selectable_label(false, stream_name).clicked()
                    {
                        *action = Some(SidebarAction::OpenTab(Box::new(TabKind::Stream {
                            connection_id: id,
                            connection_name: name.to_string(),
                            stream_name: stream_name.to_string(),
                            state: StreamState {
                                info: Some(info.clone()),
                                ..Default::default()
                            },
                        })));
                    }
                }
            }
        });
}

fn render_kv_section(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    name: &str,
    action: &mut Option<SidebarAction>,
) {
    egui::CollapsingHeader::new(t("sidebar.section_kv"))
        .id_salt(format!("kv_{id}"))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .small_button("＋")
                    .on_hover_text(t("kv.create_bucket"))
                    .clicked()
                {
                    *action = Some(SidebarAction::OpenKvBucketCreate(id));
                }
                if ui.small_button("↻").on_hover_text(t("kv.refresh")).clicked() {
                    app.backend
                        .send(BackendCommand::ListKvBuckets { connection_id: id });
                }
            });

            if let Some(infos) = app.kv_lists.get(&id) {
                for info in infos {
                    if let Some(bucket_name) = info["bucket"].as_str()
                        && ui.selectable_label(false, bucket_name).clicked()
                    {
                        *action = Some(SidebarAction::OpenTab(Box::new(TabKind::KvBucket {
                            connection_id: id,
                            connection_name: name.to_string(),
                            bucket_name: bucket_name.to_string(),
                            state: KvBucketState {
                                info: Some(info.clone()),
                                loading_entries: true,
                                ..Default::default()
                            },
                        })));
                    }
                }
            }
        });
}

fn render_selected_connection_actions(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    if let Some(id) = app.selected_conn {
        ui.separator();
        ui.add_space(4.0);
        let cfg_clone = app.config.connections.iter().find(|c| c.id == id).cloned();
        ui.horizontal(|ui| {
            if ui.small_button(t("common.edit")).clicked()
                && let Some(cfg) = &cfg_clone
            {
                app.open_edit_editor(cfg);
            }
            if ui.small_button(t("common.delete")).clicked() {
                app.editor.delete_confirm = Some(id);
            }
        });
    }
}

fn apply_sidebar_action(app: &mut EasyNatsApp, action: Option<SidebarAction>) {
    match action {
        Some(SidebarAction::Select(id)) => app.selected_conn = Some(id),
        Some(SidebarAction::Connect(id)) => app.connect(id),
        Some(SidebarAction::Disconnect(id)) => app.disconnect(id),
        Some(SidebarAction::OpenTab(tab)) => app.open_tab(*tab),
        Some(SidebarAction::OpenStreamCreate(connection_id)) => {
            app.stream_editor = StreamCreateEditor {
                visible: true,
                connection_id,
                ..Default::default()
            };
        }
        Some(SidebarAction::OpenKvBucketCreate(connection_id)) => {
            app.kv_bucket_editor = KvBucketCreateEditor {
                visible: true,
                connection_id,
                ..Default::default()
            };
        }
        None => {}
    }
}
