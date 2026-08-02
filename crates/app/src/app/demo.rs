use std::collections::{HashMap, HashSet};

use egui_dock::DockState;
use nats_backend::{
    AppConfig, AuthMethod, BackendCommand, ConnectionConfig, ConnectionStatusKind, MonitoringConfig,
};

use crate::i18n::{self, Language};
use crate::log_layer::SharedLogBuffer;
use crate::runtime::RuntimeMode;
use crate::schema::MessageSchemaManager;
use crate::settings::AppSettings;
use crate::tabs::types::SubjectSubscription;
use crate::tabs::{
    KvBucketState, SearchField, SearchResultKey, SearchResultLocator, SearchSourceId,
    SearchWorkspaceResult, SearchWorkspaceState, StreamState, TabGuard, TabKind,
};
use crate::theme::ThemeId;
use crate::toast::Toasts;
use tokio_util::sync::CancellationToken;

use super::actions::PubSubTabKind;
use super::editors::{
    ConnectionEditor, ConsumerCreateEditor, ConsumerEditEditor, KvBucketCreateEditor,
    KvBucketEditEditor, KvEntryCreateEditor, ObjStoreBucketCreateEditor, StreamCreateEditor,
    StreamPublishEditor,
};
use super::model::{EasyNatsApp, TabIdAllocator};

const DEMO_CONNECTION_ID: u64 = 1;

impl EasyNatsApp {
    pub fn new_demo(settings: AppSettings, theme_id: ThemeId) -> Self {
        let connection = ConnectionConfig {
            id: DEMO_CONNECTION_ID,
            name: "Demo".into(),
            urls: vec!["nats://demo.invalid:4222".into()],
            auth: AuthMethod::None,
            tls_enabled: false,
            tls_first: false,
            monitoring: Some(MonitoringConfig {
                endpoint: "https://demo.invalid:8222".into(),
            }),
        };
        let mut app = Self {
            backend: nats_backend::BackendHandle::spawn(),
            config: AppConfig {
                connections: vec![connection],
                next_id: 2,
            },
            settings,
            conn_statuses: [(DEMO_CONNECTION_ID, ConnectionStatusKind::Connected)].into(),
            user_wants_connected: HashSet::from([DEMO_CONNECTION_ID]),
            selected_conn: Some(DEMO_CONNECTION_ID),
            editor: ConnectionEditor::default(),
            stream_editor: StreamCreateEditor::default(),
            stream_publish_editor: StreamPublishEditor::default(),
            consumer_editor: ConsumerCreateEditor::default(),
            consumer_edit_editor: ConsumerEditEditor::default(),
            kv_bucket_editor: KvBucketCreateEditor::default(),
            kv_bucket_edit_editor: KvBucketEditEditor::default(),
            kv_entry_create_editor: KvEntryCreateEditor::default(),
            obj_store_bucket_editor: ObjStoreBucketCreateEditor::default(),
            stream_lists: HashMap::new(),
            kv_lists: HashMap::new(),
            obj_store_lists: HashMap::new(),
            dock_state: DockState::new(Vec::new()),
            toasts: Toasts::default(),
            theme_id,
            kv_bucket_delete_confirm: None,
            obj_store_bucket_delete_confirm: None,
            tab_id_alloc: TabIdAllocator::default(),
            log_buffer: SharedLogBuffer::default(),
            schema_manager: MessageSchemaManager::default(),
            runtime_mode: RuntimeMode::InteractiveDemo,
        };
        app.initialize_demo_workspace();
        app
    }

    fn initialize_demo_workspace(&mut self) {
        let mut publisher = self.new_pubsub_tab(DEMO_CONNECTION_ID, PubSubTabKind::Publisher);
        let mut subscriber = self.new_pubsub_tab(DEMO_CONNECTION_ID, PubSubTabKind::Subscriber);

        if let TabKind::Publisher { state, .. } = &mut publisher {
            state.subject = "orders.created".into();
            state.payload =
                r#"{"order_id":"ord-2001","customer":"Demo","total":99.00,"status":"created"}"#
                    .into();
        }
        if let TabKind::Subscriber {
            backend_id,
            guard,
            state,
            ..
        } = &mut subscriber
        {
            let subject = "orders.>".to_string();
            self.backend.send(BackendCommand::Subscribe {
                connection_id: DEMO_CONNECTION_ID,
                backend_id: *backend_id,
                subject: subject.clone(),
                cancel: guard.cancellation(),
            });
            state.subject_input = subject.clone();
            state.subscriptions.push(SubjectSubscription {
                subject,
                active: true,
            });
        }
        let _ = self.backend.drain_events();

        self.dock_state = DockState::new(vec![TabKind::SearchWorkspace {
            state: SearchWorkspaceState {
                query: "order".into(),
                ..Default::default()
            },
        }]);

        for stream_name in ["ORDERS", "AUDIT_LOG", "TELEMETRY"] {
            self.open_demo_stream(stream_name);
        }
        for bucket_name in ["app_config", "service_registry"] {
            self.open_demo_kv_bucket(bucket_name);
        }

        self.backend.send(BackendCommand::ListStreams {
            connection_id: DEMO_CONNECTION_ID,
        });
        self.backend.send(BackendCommand::ListKvBuckets {
            connection_id: DEMO_CONNECTION_ID,
        });
        self.backend.send(BackendCommand::ListObjectStoreBuckets {
            connection_id: DEMO_CONNECTION_ID,
        });

        let pubsub_window = self.dock_state.add_window(vec![subscriber, publisher]);
        self.dock_state
            .get_window_state_mut(pubsub_window)
            .expect("new demo window should have window state")
            .set_position(eframe::egui::pos2(650.0, 110.0))
            .set_size(eframe::egui::vec2(520.0, 460.0));

        self.configure_demo_search_workspace();
        if let Some(path) = self
            .dock_state
            .find_tab_from(|tab| matches!(tab, TabKind::SearchWorkspace { .. }))
        {
            let _ = self.dock_state.set_active_tab(path);
        }
    }

    fn open_demo_stream(&mut self, stream_name: &str) {
        self.open_tab(TabKind::Stream {
            connection_id: DEMO_CONNECTION_ID,
            connection_name: "Demo".into(),
            stream_name: stream_name.into(),
            guard: TabGuard::new_without_id(CancellationToken::new()),
            state: StreamState {
                fetching: true,
                ..Default::default()
            },
        });
        self.backend.send(BackendCommand::GetStreamMessages {
            connection_id: DEMO_CONNECTION_ID,
            stream: stream_name.into(),
            start_sequence: None,
            subject_filter: None,
            start_time: None,
            batch_size: 50,
        });
    }

    fn open_demo_kv_bucket(&mut self, bucket_name: &str) {
        self.open_tab(TabKind::KvBucket {
            connection_id: DEMO_CONNECTION_ID,
            connection_name: "Demo".into(),
            bucket_name: bucket_name.into(),
            guard: TabGuard::new_without_id(CancellationToken::new()),
            state: KvBucketState::default(),
        });
    }

    fn configure_demo_search_workspace(&mut self) {
        let sources = self.search_source_summaries();
        let orders_source_id = SearchSourceId::Stream {
            connection_id: DEMO_CONNECTION_ID,
            stream_name: "ORDERS".into(),
        };
        let source_label = sources
            .iter()
            .find(|source| source.id == orders_source_id)
            .map(|source| source.label.clone())
            .unwrap_or_else(|| "ORDERS (Demo)".into());
        let selected_result = SearchResultKey {
            source_id: orders_source_id.clone(),
            field: SearchField::Subject,
            item_id: "1".into(),
        };
        let preview = SearchWorkspaceResult {
            key: selected_result.clone(),
            source_label,
            field: SearchField::Subject,
            item_label: "#1 orders.created".into(),
            snippet: "orders.created".into(),
            preview_bytes: Some(
                br#"{"order_id":"ord-1001","customer":"Acme","total":149.50,"status":"created"}"#
                    .to_vec(),
            ),
            locator: SearchResultLocator::StreamMessage {
                connection_id: DEMO_CONNECTION_ID,
                stream_name: "ORDERS".into(),
                sequence: 1,
            },
        };

        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::SearchWorkspace { state } = tab {
                state.selected_sources = sources.iter().map(|source| source.id.clone()).collect();
                state.selected_result = Some(selected_result);
                state.selected_preview = Some(preview);
                break;
            }
        }
    }

    pub(crate) fn demo_language(&self) -> Language {
        self.settings.language
    }

    pub(crate) fn apply_demo_language(&mut self, language: Language) {
        self.settings.language = language;
        i18n::set_language(language);
    }

    pub(crate) fn demo_theme(&self) -> ThemeId {
        self.theme_id
    }

    pub(crate) fn apply_demo_theme(&mut self, theme_id: ThemeId) {
        self.settings.theme = Some(theme_id);
        self.theme_id = theme_id;
    }
}
