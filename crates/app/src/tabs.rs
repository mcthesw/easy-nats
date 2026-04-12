use eframe::egui;
use egui::WidgetText;
use egui_dock::TabViewer;

use crate::ui_strings;

/// Represents tabs in the dock area.
#[derive(Debug)]
#[allow(dead_code)]
pub enum TabKind {
    Welcome,
    Publisher {
        connection_id: u64,
        connection_name: String,
    },
    Subscriber {
        connection_id: u64,
        connection_name: String,
    },
    Stream {
        connection_id: u64,
        connection_name: String,
        stream_name: String,
    },
    KvBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
    },
    ObjectStoreBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
    },
}

impl TabKind {
    /// Format: "ResourceName (ServerName)"
    pub fn title(&self) -> String {
        match self {
            TabKind::Welcome => ui_strings::TAB_WELCOME.to_string(),
            TabKind::Publisher {
                connection_name, ..
            } => format!("{} ({})", ui_strings::TAB_PUBLISHER, connection_name),
            TabKind::Subscriber {
                connection_name, ..
            } => format!("{} ({})", ui_strings::TAB_SUBSCRIBER, connection_name),
            TabKind::Stream {
                connection_name,
                stream_name,
                ..
            } => format!("{stream_name} ({connection_name})"),
            TabKind::KvBucket {
                connection_name,
                bucket_name,
                ..
            } => format!("{bucket_name} ({connection_name})"),
            TabKind::ObjectStoreBucket {
                connection_name,
                bucket_name,
                ..
            } => format!("{bucket_name} ({connection_name})"),
        }
    }
}

/// Viewer that renders each tab type's content.
pub struct AppTabViewer;

impl TabViewer for AppTabViewer {
    type Tab = TabKind;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            TabKind::Welcome => {
                ui.heading(ui_strings::WELCOME_HEADING);
                ui.label(ui_strings::WELCOME_BODY);
            }
            TabKind::Publisher { .. } => {
                ui.label("Publisher tab — coming soon");
            }
            TabKind::Subscriber { .. } => {
                ui.label("Subscriber tab — coming soon");
            }
            TabKind::Stream { stream_name, .. } => {
                ui.label(format!("Stream: {stream_name} — coming soon"));
            }
            TabKind::KvBucket { bucket_name, .. } => {
                ui.label(format!("KV Bucket: {bucket_name} — coming soon"));
            }
            TabKind::ObjectStoreBucket { bucket_name, .. } => {
                ui.label(format!("Object Store: {bucket_name} — coming soon"));
            }
        }
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, TabKind::Welcome)
    }
}
