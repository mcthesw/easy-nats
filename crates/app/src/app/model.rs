use std::collections::HashMap;

use egui_dock::DockState;
use nats_backend::{AppConfig, ConnectionStatusKind};

use crate::log_layer::SharedLogBuffer;
use crate::proto::ProtoSchemaManager;
use crate::settings::AppSettings;
use crate::tabs::TabKind;
use crate::toast::Toasts;

use super::editors::{
    ConnectionEditor, ConsumerCreateEditor, KvBucketCreateEditor, ObjStoreBucketCreateEditor,
    StreamCreateEditor,
};

pub struct EasyNatsApp {
    pub(crate) backend: nats_backend::BackendHandle,
    pub(crate) config: AppConfig,
    pub(crate) settings: AppSettings,
    pub(crate) conn_statuses: HashMap<u64, ConnectionStatusKind>,
    pub(crate) selected_conn: Option<u64>,
    pub(crate) editor: ConnectionEditor,
    pub(crate) stream_editor: StreamCreateEditor,
    pub(crate) consumer_editor: ConsumerCreateEditor,
    pub(crate) kv_bucket_editor: KvBucketCreateEditor,
    pub(crate) obj_store_bucket_editor: ObjStoreBucketCreateEditor,
    pub(crate) stream_lists: HashMap<u64, Vec<serde_json::Value>>,
    pub(crate) kv_lists: HashMap<u64, Vec<serde_json::Value>>,
    pub(crate) obj_store_lists: HashMap<u64, Vec<serde_json::Value>>,
    pub(crate) dock_state: DockState<TabKind>,
    pub(crate) toasts: Toasts,
    pub(crate) dark_mode: bool,
    pub(crate) kv_bucket_delete_confirm: Option<(u64, String)>,
    pub(crate) obj_store_bucket_delete_confirm: Option<(u64, String)>,
    pub(crate) next_tab_instance: u32,
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
            selected_conn: None,
            editor: ConnectionEditor::default(),
            stream_editor: StreamCreateEditor::default(),
            consumer_editor: ConsumerCreateEditor::default(),
            kv_bucket_editor: KvBucketCreateEditor::default(),
            obj_store_bucket_editor: ObjStoreBucketCreateEditor::default(),
            stream_lists: HashMap::new(),
            kv_lists: HashMap::new(),
            obj_store_lists: HashMap::new(),
            dock_state: DockState::new(vec![TabKind::Welcome]),
            toasts: Toasts::default(),
            dark_mode,
            kv_bucket_delete_confirm: None,
            obj_store_bucket_delete_confirm: None,
            next_tab_instance: 1,
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
