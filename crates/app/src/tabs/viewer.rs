use eframe::egui;

use crate::tabs::{kv_bucket_ui, publisher_ui, stream_ui, subscriber_ui};
use crate::ui_strings;

use super::types::{AppTabViewer, TabKind};

pub(crate) fn render_tab(viewer: &mut AppTabViewer<'_>, ui: &mut egui::Ui, tab: &mut TabKind) {
    match tab {
        TabKind::Welcome => {
            ui.heading(ui_strings::WELCOME_HEADING);
            ui.label(ui_strings::WELCOME_BODY);
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
            ui.label(format!("Object Store: {bucket_name} — coming soon"));
        }
    }
}
