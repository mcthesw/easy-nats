use eframe::egui;

use crate::tabs::{kv_bucket_ui, publisher_ui, stream_ui, subscriber_ui};
use crate::i18n::t;

use super::types::{AppTabViewer, TabAction, TabKind};

pub(crate) fn render_tab(viewer: &mut AppTabViewer<'_>, ui: &mut egui::Ui, tab: &mut TabKind) {
    match tab {
        TabKind::Welcome => {
            let available = ui.available_size();
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    ui.cursor().min,
                    available,
                )),
                |ui| {
                    ui.vertical_centered(|ui| {
                        let top_pad = (available.y * 0.25).max(40.0);
                        ui.add_space(top_pad);

                        ui.heading(
                            egui::RichText::new("Easy NATS")
                                .size(32.0)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                .weak(),
                        );
                        ui.add_space(16.0);
                        ui.label(t("common.welcome_body"));
                        ui.add_space(24.0);
                        if ui
                            .button(
                                egui::RichText::new(t("common.welcome_new_conn"))
                                    .size(16.0),
                            )
                            .clicked()
                        {
                            viewer
                                .actions
                                .push(TabAction::OpenConnectionEditor);
                        }
                    });
                },
            );
        }
        TabKind::Publisher {
            connection_id,
            state,
            ..
        } => {
            publisher_ui(ui, *connection_id, state, viewer.backend);
        }
        TabKind::Subscriber {
            connection_id,
            state,
            ..
        } => {
            subscriber_ui(ui, *connection_id, state, viewer.backend);
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
            );
        }
        TabKind::KvBucket {
            connection_id,
            bucket_name,
            state,
            ..
        } => {
            kv_bucket_ui(
                ui,
                *connection_id,
                bucket_name,
                state,
                viewer.backend,
                viewer.actions,
            );
        }
        TabKind::ObjectStoreBucket { bucket_name, .. } => {
            ui.heading(format!("Object Store: {bucket_name}"));
            ui.label(t("common.object_store_wip"));
        }
    }
}
