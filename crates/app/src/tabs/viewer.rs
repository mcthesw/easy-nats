use eframe::egui;

use crate::i18n::t;
use crate::tabs::{
    kv_bucket_ui, metrics_ui, obj_store_bucket_ui, publisher_ui, server_info_ui, stream_ui,
    subscriber_ui,
};

use super::log_viewer::log_viewer_ui;
use super::settings::settings_ui;
use super::types::{AppTabViewer, TabAction, TabKind};

pub(crate) fn render_tab(viewer: &mut AppTabViewer<'_>, ui: &mut egui::Ui, tab: &mut TabKind) {
    ui.push_id(tab_scope_id(tab), |ui| {
        render_tab_body(viewer, ui, tab);
    });
}

fn tab_scope_id(tab: &TabKind) -> String {
    match tab {
        TabKind::Welcome => "welcome".into(),
        TabKind::Publisher {
            connection_id,
            backend_id,
            ..
        } => format!("pub:{connection_id}:{backend_id}"),
        TabKind::Subscriber {
            connection_id,
            backend_id,
            ..
        } => format!("sub:{connection_id}:{backend_id}"),
        TabKind::Stream {
            connection_id,
            stream_name,
            ..
        } => format!("stream:{connection_id}:{stream_name}"),
        TabKind::KvBucket {
            connection_id,
            bucket_name,
            ..
        } => format!("kv:{connection_id}:{bucket_name}"),
        TabKind::ObjectStoreBucket {
            connection_id,
            bucket_name,
            ..
        } => format!("obj:{connection_id}:{bucket_name}"),
        TabKind::ServerInfo { connection_id, .. } => format!("srvinfo:{connection_id}"),
        TabKind::Metrics { connection_id, .. } => format!("metrics:{connection_id}"),
        TabKind::Settings => "settings".into(),
        TabKind::LogViewer => "log-viewer".into(),
    }
}

fn render_tab_body(viewer: &mut AppTabViewer<'_>, ui: &mut egui::Ui, tab: &mut TabKind) {
    match tab {
        TabKind::Welcome => {
            let available = ui.available_size();
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(ui.cursor().min, available)),
                |ui| {
                    ui.vertical_centered(|ui| {
                        let top_pad = (available.y * 0.25).max(40.0);
                        ui.add_space(top_pad);

                        ui.heading(egui::RichText::new("Easy NATS").size(32.0).strong());
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).weak(),
                        );
                        ui.add_space(16.0);
                        ui.label(t("common.welcome_body"));
                        ui.add_space(24.0);
                        if ui
                            .button(egui::RichText::new(t("common.welcome_new_conn")).size(16.0))
                            .clicked()
                        {
                            viewer.actions.push(TabAction::OpenConnectionEditor);
                        }
                    });
                },
            );
        }
        TabKind::Publisher {
            connection_id,
            backend_id,
            state,
            ..
        } => {
            let suggestions = viewer.settings.topic_suggestions(&state.subject);
            publisher_ui(
                ui,
                *connection_id,
                *backend_id,
                state,
                viewer.backend,
                viewer.proto_manager,
                viewer.actions,
                &suggestions,
            );
        }
        TabKind::Subscriber {
            connection_id,
            backend_id,
            guard,
            state,
            ..
        } => {
            let suggestions = viewer.settings.topic_suggestions(&state.subject_input);
            subscriber_ui(
                ui,
                *connection_id,
                *backend_id,
                guard,
                state,
                viewer.backend,
                viewer.proto_manager,
                viewer.actions,
                &suggestions,
            );
        }
        TabKind::Stream {
            connection_id,
            stream_name,
            state,
            ..
        } => {
            stream_ui(
                ui,
                *connection_id,
                stream_name,
                state,
                viewer.backend,
                viewer.actions,
                viewer.proto_manager,
            );
        }
        TabKind::KvBucket {
            connection_id,
            bucket_name,
            state,
            guard,
            ..
        } => {
            kv_bucket_ui(
                ui,
                *connection_id,
                bucket_name,
                state,
                viewer.backend,
                viewer.actions,
                viewer.proto_manager,
                guard,
            );
        }
        TabKind::ObjectStoreBucket {
            connection_id,
            bucket_name,
            state,
            ..
        } => {
            obj_store_bucket_ui(
                ui,
                *connection_id,
                bucket_name,
                state,
                viewer.backend,
                viewer.actions,
            );
        }
        TabKind::ServerInfo {
            connection_id,
            state,
            ..
        } => {
            server_info_ui(ui, *connection_id, state, viewer.backend);
        }
        TabKind::Metrics {
            connection_id,
            state,
            ..
        } => {
            metrics_ui(ui, *connection_id, state, viewer.backend);
        }
        TabKind::Settings => {
            settings_ui(ui, viewer.settings, viewer.theme_id, viewer.actions);
        }
        TabKind::LogViewer => {
            log_viewer_ui(ui, viewer.log_buffer);
        }
    }
}
