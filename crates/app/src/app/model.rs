use std::collections::{BTreeSet, HashMap, HashSet};

use egui_dock::DockState;
use nats_backend::{
    AppConfig, ConnectionStatusKind, KvBucketInfo, ObjectStoreBucketInfo, StreamInfo,
};

use crate::log_layer::SharedLogBuffer;
use crate::runtime::RuntimeMode;
use crate::schema::MessageSchemaManager;
use crate::settings::AppSettings;
use crate::tabs::TabKind;
use crate::theme::ThemeId;
use crate::toast::Toasts;

use super::editors::{
    ConnectionEditor, ConsumerCreateEditor, ConsumerEditEditor, KvBucketCreateEditor,
    KvBucketEditEditor, KvEntryCreateEditor, ObjStoreBucketCreateEditor, StreamCreateEditor,
    StreamPublishEditor,
};

/// Allocates and recycles tab instance IDs, reusing the smallest freed ID.
/// IDs are returned via mpsc channel (from `TabGuard::drop`) and drained on `allocate()`.
pub struct TabIdAllocator {
    next: u32,
    free: BTreeSet<u32>,
    return_tx: std::sync::mpsc::Sender<u32>,
    return_rx: std::sync::mpsc::Receiver<u32>,
}

impl Default for TabIdAllocator {
    fn default() -> Self {
        let (return_tx, return_rx) = std::sync::mpsc::channel();
        Self {
            next: 1,
            free: BTreeSet::new(),
            return_tx,
            return_rx,
        }
    }
}

impl TabIdAllocator {
    /// Allocate a display ID and return it along with a sender for returning the ID.
    pub fn allocate(&mut self) -> (u32, std::sync::mpsc::Sender<u32>) {
        // Drain any returned IDs from TabGuard drops
        while let Ok(id) = self.return_rx.try_recv() {
            self.free.insert(id);
        }
        let id = if let Some(&id) = self.free.iter().next() {
            self.free.remove(&id);
            id
        } else {
            let id = self.next;
            self.next += 1;
            id
        };
        (id, self.return_tx.clone())
    }
}

pub struct EasyNatsApp {
    pub(crate) backend: nats_backend::BackendHandle,
    pub(crate) config: AppConfig,
    pub(crate) settings: AppSettings,
    pub(crate) conn_statuses: HashMap<u64, ConnectionStatusKind>,
    /// Tracks explicit user intent: connections the user asked to keep alive.
    pub(crate) user_wants_connected: HashSet<u64>,
    pub(crate) selected_conn: Option<u64>,
    pub(crate) editor: ConnectionEditor,
    pub(crate) stream_editor: StreamCreateEditor,
    pub(crate) stream_publish_editor: StreamPublishEditor,
    pub(crate) consumer_editor: ConsumerCreateEditor,
    pub(crate) consumer_edit_editor: ConsumerEditEditor,
    pub(crate) kv_bucket_editor: KvBucketCreateEditor,
    pub(crate) kv_bucket_edit_editor: KvBucketEditEditor,
    pub(crate) kv_entry_create_editor: KvEntryCreateEditor,
    pub(crate) obj_store_bucket_editor: ObjStoreBucketCreateEditor,
    pub(crate) stream_lists: HashMap<u64, Vec<StreamInfo>>,
    pub(crate) kv_lists: HashMap<u64, Vec<KvBucketInfo>>,
    pub(crate) obj_store_lists: HashMap<u64, Vec<ObjectStoreBucketInfo>>,
    pub(crate) dock_state: DockState<TabKind>,
    pub(crate) toasts: Toasts,
    pub(crate) theme_id: ThemeId,
    pub(crate) kv_bucket_delete_confirm: Option<(u64, String)>,
    pub(crate) obj_store_bucket_delete_confirm: Option<(u64, String)>,
    pub(crate) tab_id_alloc: TabIdAllocator,
    pub(crate) log_buffer: SharedLogBuffer,
    pub(crate) schema_manager: MessageSchemaManager,
    pub(crate) runtime_mode: RuntimeMode,
}

impl EasyNatsApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(settings: AppSettings, theme_id: ThemeId, log_buffer: SharedLogBuffer) -> Self {
        let schema_manager = MessageSchemaManager::load(settings.proto_schema_dir.as_deref());
        Self {
            backend: nats_backend::BackendHandle::spawn(),
            config: AppConfig::load(),
            settings,
            conn_statuses: HashMap::new(),
            user_wants_connected: HashSet::new(),
            selected_conn: None,
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
            dock_state: DockState::new(vec![TabKind::Welcome]),
            toasts: Toasts::default(),
            theme_id,
            kv_bucket_delete_confirm: None,
            obj_store_bucket_delete_confirm: None,
            tab_id_alloc: TabIdAllocator::default(),
            log_buffer,
            schema_manager,
            runtime_mode: RuntimeMode::Native,
        }
    }

    pub(crate) fn conn_name(&self, id: u64) -> String {
        self.config
            .connections
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("#{id}"))
    }

    pub(crate) fn connection_metrics_endpoint(&self, id: u64) -> Option<String> {
        self.config
            .connections
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.monitoring_endpoint())
            .map(ToOwned::to_owned)
    }
}
