use std::collections::{BTreeSet, HashMap, HashSet};

use egui_dock::DockState;
use nats_backend::{AppConfig, ConnectionStatusKind};

use crate::log_layer::SharedLogBuffer;
use crate::proto::ProtoSchemaManager;
use crate::settings::AppSettings;
use crate::tabs::TabKind;
use crate::toast::Toasts;

use super::editors::{
    ConnectionEditor, ConsumerCreateEditor, ConsumerEditEditor, KvBucketCreateEditor,
    KvBucketEditEditor, KvEntryCreateEditor, ObjStoreBucketCreateEditor, StreamCreateEditor,
    StreamPublishEditor,
};

/// Allocates and recycles tab instance IDs, reusing the smallest freed ID.
pub struct TabIdAllocator {
    next: u32,
    free: BTreeSet<u32>,
}

impl Default for TabIdAllocator {
    fn default() -> Self {
        Self {
            next: 1,
            free: BTreeSet::new(),
        }
    }
}

impl TabIdAllocator {
    pub fn allocate(&mut self) -> u32 {
        if let Some(&id) = self.free.iter().next() {
            self.free.remove(&id);
            id
        } else {
            let id = self.next;
            self.next += 1;
            id
        }
    }

    pub fn free(&mut self, id: u32) {
        self.free.insert(id);
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
    pub(crate) stream_lists: HashMap<u64, Vec<serde_json::Value>>,
    pub(crate) kv_lists: HashMap<u64, Vec<serde_json::Value>>,
    pub(crate) obj_store_lists: HashMap<u64, Vec<serde_json::Value>>,
    pub(crate) dock_state: DockState<TabKind>,
    pub(crate) toasts: Toasts,
    pub(crate) dark_mode: bool,
    pub(crate) kv_bucket_delete_confirm: Option<(u64, String)>,
    pub(crate) obj_store_bucket_delete_confirm: Option<(u64, String)>,
    pub(crate) tab_id_alloc: TabIdAllocator,
    pub(crate) log_buffer: SharedLogBuffer,
    pub(crate) proto_manager: ProtoSchemaManager,
}

impl EasyNatsApp {
    pub fn new(dark_mode: bool, log_buffer: SharedLogBuffer) -> Self {
        let settings = AppSettings::load();
        let mut proto_manager = ProtoSchemaManager::default();
        if let Some(dir) = &settings.proto_schema_dir {
            proto_manager.set_schema_dir(std::path::PathBuf::from(dir));
        }
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
            dark_mode,
            kv_bucket_delete_confirm: None,
            obj_store_bucket_delete_confirm: None,
            tab_id_alloc: TabIdAllocator::default(),
            log_buffer,
            proto_manager,
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
}
