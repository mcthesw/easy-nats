use eframe::egui;
use egui_dock::TabViewer;
use egui_dock::tab_viewer::OnCloseResponse;

use crate::i18n::t;
use crate::schema::MessageSchemaManager;
use crate::theme::ThemeId;

use super::{
    ClientStatusState, KvBucketState, MessageSchemasState, MetricsState, ObjectStoreBucketState,
    PublisherState, SearchSourceSummary, SearchWorkspaceState, ServerInfoState, StreamState,
    SubscriberState, TabAction,
};
use crate::tabs::guard::TabGuard;

#[derive(Debug)]
#[allow(dead_code)] // guard fields exist for RAII Drop cleanup, not direct reads
pub enum TabKind {
    Welcome,
    Publisher {
        connection_id: u64,
        connection_name: String,
        guard: TabGuard,
        backend_id: u64,
        state: PublisherState,
    },
    Subscriber {
        connection_id: u64,
        connection_name: String,
        guard: TabGuard,
        backend_id: u64,
        state: SubscriberState,
    },
    Stream {
        connection_id: u64,
        connection_name: String,
        stream_name: String,
        guard: TabGuard,
        state: StreamState,
    },
    KvBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
        guard: TabGuard,
        state: KvBucketState,
    },
    ObjectStoreBucket {
        connection_id: u64,
        connection_name: String,
        bucket_name: String,
        guard: TabGuard,
        state: ObjectStoreBucketState,
    },
    ServerInfo {
        connection_id: u64,
        connection_name: String,
        guard: TabGuard,
        state: ServerInfoState,
    },
    Metrics {
        connection_id: u64,
        connection_name: String,
        state: MetricsState,
    },
    Clients {
        connection_id: u64,
        connection_name: String,
        state: ClientStatusState,
    },
    SearchWorkspace {
        state: SearchWorkspaceState,
    },
    MessageSchemas {
        state: MessageSchemasState,
    },
    Settings,
    LogViewer,
}

impl TabKind {
    pub fn title(&self, show_connection: bool) -> String {
        match self {
            TabKind::Welcome => t("common.tab_welcome").to_string(),
            TabKind::Publisher {
                connection_name,
                guard,
                ..
            } => {
                let title = if let Some(id) = guard.display_id() {
                    format!("{} #{}", t("common.tab_publisher"), id)
                } else {
                    t("common.tab_publisher").to_string()
                };
                title_with_connection(title, connection_name, show_connection)
            }
            TabKind::Subscriber {
                connection_name,
                guard,
                ..
            } => {
                let title = if let Some(id) = guard.display_id() {
                    format!("{} #{}", t("common.tab_subscriber"), id)
                } else {
                    t("common.tab_subscriber").to_string()
                };
                title_with_connection(title, connection_name, show_connection)
            }
            TabKind::Stream {
                connection_name,
                stream_name,
                ..
            } => title_with_connection(stream_name.clone(), connection_name, show_connection),
            TabKind::KvBucket {
                connection_name,
                bucket_name,
                ..
            } => title_with_connection(bucket_name.clone(), connection_name, show_connection),
            TabKind::ObjectStoreBucket {
                connection_name,
                bucket_name,
                ..
            } => title_with_connection(bucket_name.clone(), connection_name, show_connection),
            TabKind::ServerInfo {
                connection_name, ..
            } => title_with_connection(
                t("server_info.title").to_string(),
                connection_name,
                show_connection,
            ),
            TabKind::Metrics {
                connection_name, ..
            } => title_with_connection(
                t("common.tab_metrics").to_string(),
                connection_name,
                show_connection,
            ),
            TabKind::Clients {
                connection_name, ..
            } => title_with_connection(
                t("common.tab_clients").to_string(),
                connection_name,
                show_connection,
            ),
            TabKind::SearchWorkspace { .. } => t("common.tab_search_workspace").to_string(),
            TabKind::MessageSchemas { .. } => t("common.tab_message_schemas").to_string(),
            TabKind::Settings => t("settings.title").to_string(),
            TabKind::LogViewer => t("log_viewer.title").to_string(),
        }
    }

    /// Structural identity for use in close actions and deduplication.
    pub fn tab_id(&self) -> egui::Id {
        match self {
            TabKind::Welcome => egui::Id::new("tab:welcome"),
            TabKind::Publisher {
                connection_id,
                backend_id,
                ..
            } => egui::Id::new(("tab:publisher", *connection_id, *backend_id)),
            TabKind::Subscriber {
                connection_id,
                backend_id,
                ..
            } => egui::Id::new(("tab:subscriber", *connection_id, *backend_id)),
            TabKind::Stream {
                connection_id,
                stream_name,
                ..
            } => egui::Id::new(("tab:stream", *connection_id, stream_name.as_str())),
            TabKind::KvBucket {
                connection_id,
                bucket_name,
                ..
            } => egui::Id::new(("tab:kv", *connection_id, bucket_name.as_str())),
            TabKind::ObjectStoreBucket {
                connection_id,
                bucket_name,
                ..
            } => egui::Id::new(("tab:object-store", *connection_id, bucket_name.as_str())),
            TabKind::ServerInfo { connection_id, .. } => {
                egui::Id::new(("tab:server-info", *connection_id))
            }
            TabKind::Metrics { connection_id, .. } => {
                egui::Id::new(("tab:metrics", *connection_id))
            }
            TabKind::Clients { connection_id, .. } => {
                egui::Id::new(("tab:clients", *connection_id))
            }
            TabKind::SearchWorkspace { .. } => egui::Id::new("tab:search-workspace"),
            TabKind::MessageSchemas { .. } => egui::Id::new("tab:message-schemas"),
            TabKind::Settings => egui::Id::new("tab:settings"),
            TabKind::LogViewer => egui::Id::new("tab:log-viewer"),
        }
    }

    /// The connection this tab is bound to, if any.
    ///
    /// Tabs without a backing connection (Welcome, SearchWorkspace, MessageSchemas,
    /// Settings, LogViewer) return `None`. Used for connection-scoped teardown such
    /// as closing every tab belonging to a disconnected connection.
    pub fn connection_id(&self) -> Option<u64> {
        match self {
            TabKind::Welcome
            | TabKind::SearchWorkspace { .. }
            | TabKind::MessageSchemas { .. }
            | TabKind::Settings
            | TabKind::LogViewer => None,
            TabKind::Publisher { connection_id, .. }
            | TabKind::Subscriber { connection_id, .. }
            | TabKind::Stream { connection_id, .. }
            | TabKind::KvBucket { connection_id, .. }
            | TabKind::ObjectStoreBucket { connection_id, .. }
            | TabKind::ServerInfo { connection_id, .. }
            | TabKind::Metrics { connection_id, .. }
            | TabKind::Clients { connection_id, .. } => Some(*connection_id),
        }
    }
}

pub struct AppTabViewer<'a> {
    pub backend: &'a nats_backend::BackendHandle,
    pub actions: &'a mut Vec<TabAction>,
    pub search_sources: &'a [SearchSourceSummary],
    pub settings: &'a mut crate::settings::AppSettings,
    pub theme_id: &'a mut ThemeId,
    pub log_buffer: &'a crate::log_layer::SharedLogBuffer,
    pub schema_manager: &'a MessageSchemaManager,
    pub connections: &'a [(u64, String)],
    pub runtime_mode: crate::runtime::RuntimeMode,
    pub show_connection_in_title: bool,
}

fn title_with_connection(title: String, connection_name: &str, show_connection: bool) -> String {
    if show_connection {
        format!("{title} ({connection_name})")
    } else {
        title
    }
}

impl TabViewer for AppTabViewer<'_> {
    type Tab = TabKind;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title(self.show_connection_in_title).into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        tab.tab_id()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        crate::tabs::viewer::render_tab(self, ui, tab);
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab, _path: egui_dock::NodePath) {
        let tab_id = self.id(tab);
        if ui.button(t("common.close_others")).clicked() {
            self.actions.push(TabAction::CloseOtherTabs {
                keep_tab_id: tab_id,
            });
            ui.close();
        }
        if ui.button(t("common.close_all")).clicked() {
            self.actions.push(TabAction::CloseAllTabs);
            ui.close();
        }
        if ui.button(t("common.close_to_right")).clicked() {
            self.actions
                .push(TabAction::CloseTabsToRight { of_tab_id: tab_id });
            ui.close();
        }
    }

    fn is_closeable(&self, tab: &Self::Tab) -> bool {
        !matches!(tab, TabKind::Welcome)
    }

    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, TabKind::Welcome)
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        // Send explicit Unsubscribe to clean up worker state.
        // TabGuard Drop handles cancellation + display ID recycling automatically.
        if let TabKind::Subscriber {
            connection_id,
            backend_id,
            state,
            ..
        } = tab
        {
            for sub in &state.subscriptions {
                if sub.active {
                    self.backend
                        .send(nats_backend::BackendCommand::Unsubscribe {
                            connection_id: *connection_id,
                            backend_id: *backend_id,
                            subject: sub.subject.clone(),
                        });
                }
            }
        }
        OnCloseResponse::Close
    }
}
