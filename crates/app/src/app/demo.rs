use std::collections::{HashMap, HashSet};

use egui_dock::DockState;
use nats_backend::{
    AppConfig, AuthMethod, BackendCommand, ConnectionConfig, ConnectionStatusKind, MonitoringConfig,
};

use crate::log_layer::SharedLogBuffer;
use crate::runtime::RuntimeMode;
use crate::schema::MessageSchemaManager;
use crate::settings::AppSettings;
use crate::tabs::TabKind;
use crate::tabs::types::SubjectSubscription;
use crate::theme::ThemeId;
use crate::toast::Toasts;

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
            name: "Interactive Demo".into(),
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
            state.subject = "demo.events.manual".into();
            state.payload = r#"{"source":"manual","message":"Hello from Easy NATS"}"#.into();
        }
        if let TabKind::Subscriber {
            backend_id,
            guard,
            state,
            ..
        } = &mut subscriber
        {
            let subject = "demo.events.>".to_string();
            self.backend.send(BackendCommand::Subscribe {
                connection_id: DEMO_CONNECTION_ID,
                backend_id: *backend_id,
                subject: subject.clone(),
                cancel: guard.cancellation(),
            });
            state.subscriptions.push(SubjectSubscription {
                subject,
                active: true,
            });
        }
        let _ = self.backend.drain_events();

        self.backend.send(BackendCommand::ListStreams {
            connection_id: DEMO_CONNECTION_ID,
        });
        self.backend.send(BackendCommand::ListKvBuckets {
            connection_id: DEMO_CONNECTION_ID,
        });
        self.backend.send(BackendCommand::ListObjectStoreBuckets {
            connection_id: DEMO_CONNECTION_ID,
        });

        self.dock_state = DockState::new(vec![TabKind::Welcome]);

        let subscriber_window = self.dock_state.add_window(vec![subscriber]);
        self.dock_state
            .get_window_state_mut(subscriber_window)
            .expect("new demo window should have window state")
            .set_position(eframe::egui::pos2(300.0, 280.0))
            .set_size(eframe::egui::vec2(700.0, 560.0));

        let publisher_window = self.dock_state.add_window(vec![publisher]);
        self.dock_state
            .get_window_state_mut(publisher_window)
            .expect("new demo window should have window state")
            .set_position(eframe::egui::pos2(820.0, 150.0))
            .set_size(eframe::egui::vec2(680.0, 540.0));
    }
}
