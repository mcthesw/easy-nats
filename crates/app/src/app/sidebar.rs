use eframe::egui;
use nats_backend::{BackendCommand, ConnectionStatusKind};
use tokio_util::sync::CancellationToken;

use crate::i18n::t;
use crate::tabs::{KvBucketState, ObjectStoreBucketState, StreamState, TabGuard, TabKind};

use super::{
    editors::{KvBucketCreateEditor, ObjStoreBucketCreateEditor, StreamCreateEditor},
    model::EasyNatsApp,
};

pub(crate) enum SidebarAction {
    Select(u64),
    Connect(u64),
    Disconnect(u64),
    OpenTab(Box<TabKind>),
    OpenPublisher(u64),
    OpenSubscriber(u64),
    OpenStreamCreate(u64),
    OpenKvBucketCreate(u64),
    OpenObjStoreBucketCreate(u64),
    OpenServerInfo(u64),
    OpenMetrics(u64),
    OpenClients(u64),
}

pub(crate) fn render_sidebar(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    egui::Panel::left("sidebar_panel")
        .default_size(220.0)
        .show(ui, |ui| {
            render_sidebar_header(app, ui);
            ui.separator();
            render_connection_tree(app, ui);
        });
}

fn render_sidebar_header(app: &mut EasyNatsApp, ui: &mut egui::Ui) {
    ui.heading(t("sidebar.connections_heading"));
    ui.horizontal(|ui| {
        if ui
            .small_button("＋")
            .on_hover_text(t("sidebar.connection_new"))
            .clicked()
        {
            app.open_new_editor();
        }
        if ui
            .small_button("📝")
            .on_hover_text(t("message_schema.title"))
            .clicked()
        {
            app.open_or_focus_message_schemas();
        }
        if ui
            .small_button("🔍")
            .on_hover_text(t("search_workspace.title"))
            .clicked()
        {
            app.open_or_focus_search_workspace();
        }
        if ui
            .small_button("📋")
            .on_hover_text(t("log_viewer.title"))
            .clicked()
        {
            app.open_or_focus_tab_kind(crate::tabs::TabKind::LogViewer);
        }
        if ui
            .small_button("⚙")
            .on_hover_text(t("settings.title"))
            .clicked()
        {
            app.open_or_focus_tab_kind(crate::tabs::TabKind::Settings);
        }
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

    let row_resp = ui.horizontal(|ui| {
        ui.colored_label(status_color, "●");
        if ui.selectable_label(selected, name).clicked() {
            *action = Some(SidebarAction::Select(id));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.user_wants_connected.contains(&id) {
                if ui
                    .small_button("⏏")
                    .on_hover_text(t("connection.disconnect"))
                    .clicked()
                {
                    *action = Some(SidebarAction::Disconnect(id));
                }
            } else {
                if ui
                    .small_button("▶")
                    .on_hover_text(t("connection.connect"))
                    .clicked()
                {
                    *action = Some(SidebarAction::Connect(id));
                }
            }

            if ui
                .small_button("🗑")
                .on_hover_text(t("sidebar.action_delete"))
                .clicked()
            {
                app.editor.delete_confirm = Some(id);
            }

            if ui
                .small_button("✏")
                .on_hover_text(t("sidebar.action_edit"))
                .clicked()
                && let Some(cfg) = app.config.connections.iter().find(|c| c.id == id).cloned()
            {
                app.open_edit_editor(&cfg);
            }
        });
    });

    let _ = row_resp;
}

fn render_resource_tree(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    name: &str,
    action: &mut Option<SidebarAction>,
) {
    ui.indent(format!("tree_{id}"), |ui| {
        render_pubsub_section(ui, id, action);
        render_streams_section(app, ui, id, name, action);
        render_kv_section(app, ui, id, name, action);
        render_obj_store_section(app, ui, id, name, action);
        render_metrics_entry(app, ui, id, action);
        render_clients_entry(app, ui, id, action);
        if render_sidebar_leaf(ui, "ℹ", t("server_info.title")).clicked() {
            *action = Some(SidebarAction::OpenServerInfo(id));
        }
    });
}

fn render_metrics_entry(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    action: &mut Option<SidebarAction>,
) {
    if app.connection_metrics_endpoint(id).is_none() {
        return;
    }

    if render_sidebar_leaf(ui, "◔", t("sidebar.section_metrics")).clicked() {
        *action = Some(SidebarAction::OpenMetrics(id));
    }
}

fn render_clients_entry(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    action: &mut Option<SidebarAction>,
) {
    if app.connection_metrics_endpoint(id).is_none() {
        return;
    }

    if render_sidebar_leaf(ui, "@", t("sidebar.section_clients")).clicked() {
        *action = Some(SidebarAction::OpenClients(id));
    }
}

fn render_sidebar_leaf(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let icon_response = ui.add_sized(
            [16.0, ui.spacing().interact_size.y],
            egui::Label::new(egui::RichText::new(icon).weak()).sense(egui::Sense::click()),
        );
        let label_response = ui.selectable_label(false, label);
        icon_response.union(label_response)
    })
    .inner
}

fn render_pubsub_section(ui: &mut egui::Ui, id: u64, action: &mut Option<SidebarAction>) {
    egui::CollapsingHeader::new(t("sidebar.section_pubsub"))
        .id_salt(format!("pubsub_{id}"))
        .show(ui, |ui| {
            if ui
                .selectable_label(false, t("sidebar.open_publisher"))
                .clicked()
            {
                *action = Some(SidebarAction::OpenPublisher(id));
            }
            if ui
                .selectable_label(false, t("sidebar.open_subscriber"))
                .clicked()
            {
                *action = Some(SidebarAction::OpenSubscriber(id));
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
                    let stream_name = info.name.as_str();
                    if !stream_visible_in_sidebar(
                        stream_name,
                        app.settings.show_backing_streams_in_sidebar,
                    ) {
                        continue;
                    }
                    if ui.selectable_label(false, stream_name).clicked() {
                        let cancel = CancellationToken::new();
                        let guard = TabGuard::new_without_id(cancel);
                        *action = Some(SidebarAction::OpenTab(Box::new(TabKind::Stream {
                            connection_id: id,
                            connection_name: name.to_string(),
                            stream_name: stream_name.to_string(),
                            guard,
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

fn stream_visible_in_sidebar(stream_name: &str, show_backing_streams: bool) -> bool {
    show_backing_streams || !is_backing_stream_name(stream_name)
}

fn is_backing_stream_name(stream_name: &str) -> bool {
    stream_name.starts_with("KV_") || stream_name.starts_with("OBJ_")
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
                if ui
                    .small_button("↻")
                    .on_hover_text(t("kv.refresh"))
                    .clicked()
                {
                    app.backend
                        .send(BackendCommand::ListKvBuckets { connection_id: id });
                }
            });

            if let Some(infos) = app.kv_lists.get(&id) {
                for info in infos {
                    let bucket_name = info.bucket.as_str();
                    if ui.selectable_label(false, bucket_name).clicked() {
                        let cancel = CancellationToken::new();
                        let guard = TabGuard::new_without_id(cancel);
                        *action = Some(SidebarAction::OpenTab(Box::new(TabKind::KvBucket {
                            connection_id: id,
                            connection_name: name.to_string(),
                            bucket_name: bucket_name.to_string(),
                            guard,
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

fn render_obj_store_section(
    app: &mut EasyNatsApp,
    ui: &mut egui::Ui,
    id: u64,
    name: &str,
    action: &mut Option<SidebarAction>,
) {
    egui::CollapsingHeader::new(t("sidebar.section_object_store"))
        .id_salt(format!("objstore_{id}"))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .small_button("＋")
                    .on_hover_text(t("obj_store.create_bucket"))
                    .clicked()
                {
                    *action = Some(SidebarAction::OpenObjStoreBucketCreate(id));
                }
                if ui
                    .small_button("↻")
                    .on_hover_text(t("obj_store.refresh"))
                    .clicked()
                {
                    app.backend
                        .send(BackendCommand::ListObjectStoreBuckets { connection_id: id });
                }
            });

            if let Some(infos) = app.obj_store_lists.get(&id) {
                for info in infos {
                    let bucket_name = info.bucket.as_str();
                    if ui.selectable_label(false, bucket_name).clicked() {
                        let cancel = CancellationToken::new();
                        let guard = TabGuard::new_without_id(cancel);
                        *action = Some(SidebarAction::OpenTab(Box::new(
                            TabKind::ObjectStoreBucket {
                                connection_id: id,
                                connection_name: name.to_string(),
                                bucket_name: bucket_name.to_string(),
                                guard,
                                state: ObjectStoreBucketState {
                                    info: Some(info.clone()),
                                    loading_objects: true,
                                    ..Default::default()
                                },
                            },
                        )));
                    }
                }
            }
        });
}

fn apply_sidebar_action(app: &mut EasyNatsApp, action: Option<SidebarAction>) {
    match action {
        Some(SidebarAction::Select(id)) => app.selected_conn = Some(id),
        Some(SidebarAction::Connect(id)) => app.connect(id),
        Some(SidebarAction::Disconnect(id)) => app.disconnect(id),
        Some(SidebarAction::OpenTab(tab)) => app.open_tab(*tab),
        Some(SidebarAction::OpenPublisher(connection_id)) => {
            app.open_or_focus_publisher_tab(connection_id);
        }
        Some(SidebarAction::OpenSubscriber(connection_id)) => {
            app.open_or_focus_subscriber_tab(connection_id);
        }
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
        Some(SidebarAction::OpenObjStoreBucketCreate(connection_id)) => {
            app.obj_store_bucket_editor = ObjStoreBucketCreateEditor {
                visible: true,
                connection_id,
                ..Default::default()
            };
        }
        Some(SidebarAction::OpenServerInfo(id)) => {
            let conn_name = app.conn_name(id);
            let cancel = CancellationToken::new();
            let guard = TabGuard::new_without_id(cancel);
            let tab = TabKind::ServerInfo {
                connection_id: id,
                connection_name: conn_name,
                guard,
                state: crate::tabs::ServerInfoState {
                    loading: true,
                    ..Default::default()
                },
            };
            app.open_or_focus_tab_kind(tab);
            app.backend
                .send(BackendCommand::GetServerInfo { connection_id: id });
            app.backend
                .send(BackendCommand::GetJetStreamAccountInfo { connection_id: id });
        }
        Some(SidebarAction::OpenMetrics(id)) => {
            app.open_or_focus_metrics_tab(id);
        }
        Some(SidebarAction::OpenClients(id)) => {
            app.open_or_focus_clients_tab(id);
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::stream_visible_in_sidebar;

    #[test]
    fn backing_streams_are_hidden_by_default() {
        assert!(!stream_visible_in_sidebar("KV_orders", false));
        assert!(!stream_visible_in_sidebar("OBJ_reports", false));
        assert!(stream_visible_in_sidebar("ORDERS", false));
    }

    #[test]
    fn backing_streams_are_visible_when_opted_in() {
        assert!(stream_visible_in_sidebar("KV_orders", true));
        assert!(stream_visible_in_sidebar("OBJ_reports", true));
        assert!(stream_visible_in_sidebar("ORDERS", true));
    }
}
