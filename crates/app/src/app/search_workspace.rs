use nats_backend::BackendCommand;

use crate::i18n::t;
use crate::tabs::{
    KV_VALUE_SEARCH_BATCH, SearchResultLocator, SearchSourceId, SearchSourceSnapshot,
    SearchWorkspaceState, TabKind, source_snapshot_from_tab,
};
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn search_source_snapshots(&self) -> Vec<SearchSourceSnapshot> {
        self.dock_state
            .iter_all_tabs()
            .filter_map(|(_, tab)| source_snapshot_from_tab(tab))
            .collect()
    }

    pub(crate) fn open_or_focus_search_workspace(&mut self) {
        self.open_or_focus_tab_kind(TabKind::SearchWorkspace {
            state: SearchWorkspaceState::default(),
        });
    }

    pub(crate) fn scan_search_workspace_kv_values(&mut self, source_id: &SearchSourceId) {
        let SearchSourceId::Kv {
            connection_id,
            bucket_name,
        } = source_id
        else {
            return;
        };

        let mut requests = Vec::new();
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::KvBucket {
                connection_id: cid,
                bucket_name: current_bucket,
                state,
                ..
            } = tab
                && *cid == *connection_id
                && current_bucket == bucket_name
            {
                if state.value_search_scanning > 0 {
                    return;
                }
                let start = state.value_search_cursor.min(state.keys.len());
                let end = (start + KV_VALUE_SEARCH_BATCH).min(state.keys.len());
                requests = state.keys[start..end]
                    .iter()
                    .filter(|key| !state.fetched_values.contains_key(key.as_str()))
                    .cloned()
                    .collect();
                state.value_search_cursor = end;
                state.value_search_pending.extend(requests.iter().cloned());
                state.value_search_scanning = state.value_search_pending.len();
                break;
            }
        }

        for key in requests {
            self.backend.send(BackendCommand::GetKvEntry {
                connection_id: *connection_id,
                bucket: bucket_name.clone(),
                key,
            });
        }
    }

    pub(crate) fn navigate_search_result(&mut self, locator: SearchResultLocator) {
        let resolved = match locator {
            SearchResultLocator::KvKey {
                connection_id,
                bucket_name,
                key,
            } => self.navigate_kv_result(connection_id, &bucket_name, &key),
            SearchResultLocator::StreamMessage {
                connection_id,
                stream_name,
                sequence,
            } => self.navigate_stream_result(connection_id, &stream_name, sequence),
            SearchResultLocator::SubscriberMessage {
                connection_id,
                backend_id,
                message_id,
            } => self.navigate_subscriber_result(connection_id, backend_id, message_id),
        };

        if !resolved {
            self.toasts.push(
                ToastLevel::Info,
                t("search_workspace.stale_result").to_string(),
            );
        }
    }

    fn navigate_kv_result(&mut self, connection_id: u64, bucket_name: &str, key: &str) -> bool {
        let Some(path) = self.dock_state.find_tab_from(|tab| {
            matches!(
                tab,
                TabKind::KvBucket {
                    connection_id: cid,
                    bucket_name: current_bucket,
                    ..
                } if *cid == connection_id && current_bucket == bucket_name
            )
        }) else {
            return false;
        };
        let _ = self.dock_state.set_active_tab(path);

        let mut found = false;
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::KvBucket {
                connection_id: cid,
                bucket_name: current_bucket,
                state,
                ..
            } = tab
                && *cid == connection_id
                && current_bucket == bucket_name
            {
                found = state.keys.iter().any(|loaded_key| loaded_key == key)
                    || state.fetched_values.contains_key(key);
                if found {
                    state.selected_key = Some(key.to_string());
                    state.show_history = false;
                    state.entry_key.clear();
                    state.entry_value.clear();
                    state.entry_revision = None;
                    state.entry_operation = None;
                    state.entry_created = None;
                    state.loading_entry = true;
                    state.loading_history = true;
                }
                break;
            }
        }

        if found {
            self.backend.send(BackendCommand::GetKvEntry {
                connection_id,
                bucket: bucket_name.to_string(),
                key: key.to_string(),
            });
            self.backend.send(BackendCommand::GetKvHistory {
                connection_id,
                bucket: bucket_name.to_string(),
                key: key.to_string(),
            });
        }
        found
    }

    fn navigate_stream_result(
        &mut self,
        connection_id: u64,
        stream_name: &str,
        sequence: u64,
    ) -> bool {
        let Some(path) = self.dock_state.find_tab_from(|tab| {
            matches!(
                tab,
                TabKind::Stream {
                    connection_id: cid,
                    stream_name: current_stream,
                    ..
                } if *cid == connection_id && current_stream == stream_name
            )
        }) else {
            return false;
        };
        let _ = self.dock_state.set_active_tab(path);

        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Stream {
                connection_id: cid,
                stream_name: current_stream,
                state,
                ..
            } = tab
                && *cid == connection_id
                && current_stream == stream_name
            {
                if let Some(idx) = state
                    .messages
                    .iter()
                    .position(|msg| msg["sequence"].as_u64() == Some(sequence))
                {
                    state.selected_msg = Some(idx);
                    return true;
                }
                return false;
            }
        }
        false
    }

    fn navigate_subscriber_result(
        &mut self,
        connection_id: u64,
        backend_id: u64,
        message_id: u64,
    ) -> bool {
        let Some(path) = self.dock_state.find_tab_from(|tab| {
            matches!(
                tab,
                TabKind::Subscriber {
                    connection_id: cid,
                    backend_id: current_backend,
                    ..
                } if *cid == connection_id && *current_backend == backend_id
            )
        }) else {
            return false;
        };
        let _ = self.dock_state.set_active_tab(path);

        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::Subscriber {
                connection_id: cid,
                backend_id: current_backend,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *current_backend == backend_id
            {
                if let Some(idx) = state
                    .messages
                    .iter()
                    .position(|message| message.id == message_id)
                {
                    state.selected_idx = Some(idx);
                    return true;
                }
                return false;
            }
        }
        false
    }
}
