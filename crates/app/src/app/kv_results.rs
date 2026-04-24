use nats_backend::{BackendCommand, BackendOperation};

use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::{model::EasyNatsApp, util::decode_kv_value};

impl EasyNatsApp {
    pub(crate) fn apply_kv_operation(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        data: &serde_json::Value,
    ) -> bool {
        match operation {
            BackendOperation::ListKvBuckets => {
                if let Some(arr) = data.as_array() {
                    let infos = arr.clone();
                    for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                        if let TabKind::KvBucket {
                            connection_id: cid,
                            bucket_name,
                            state,
                            ..
                        } = tab
                            && *cid == connection_id
                        {
                            state.info = infos
                                .iter()
                                .find(|i| i["bucket"].as_str() == Some(bucket_name.as_str()))
                                .cloned();
                        }
                    }
                    self.kv_lists.insert(connection_id, infos);
                }
                true
            }
            BackendOperation::CreateKvBucket
            | BackendOperation::DeleteKvBucket
            | BackendOperation::UpdateKvBucket => {
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                if operation == BackendOperation::DeleteKvBucket
                    && let Some(bucket) = data["bucket"].as_str()
                {
                    self.remove_tabs_matching(|tab| {
                        matches!(tab, TabKind::KvBucket { connection_id: cid, bucket_name, .. }
                            if *cid == connection_id && bucket_name == bucket)
                    });
                }
                self.backend
                    .send(BackendCommand::ListKvBuckets { connection_id });
                true
            }
            BackendOperation::ListKvKeys => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                let generation = data["generation"].as_u64().unwrap_or(0);
                let done = data["done"].as_bool().unwrap_or(true);
                let new_keys: Vec<String> = data["entries"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v["key"].as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default();

                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::KvBucket {
                        connection_id: cid,
                        bucket_name,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                        && *bucket_name == bucket
                        && state.load_generation == generation
                    {
                        state.keys.extend(new_keys.clone());
                        if done {
                            state.keys.sort();
                            state.keys_complete = true;
                            state.loading_entries = false;
                        }
                    }
                }
                true
            }
            BackendOperation::GetKvEntry => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                let entry = data["entry"].clone();
                let entry_key = entry["key"].as_str().unwrap_or("");
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::KvBucket {
                        connection_id: cid,
                        bucket_name,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                        && *bucket_name == bucket
                    {
                        let entry_value = decode_kv_value(&entry);
                        state
                            .fetched_values
                            .insert(entry_key.to_string(), entry_value.clone());
                        state.value_search_scanning = state.value_search_scanning.saturating_sub(1);
                        if state.selected_key.as_deref() == Some(entry_key) {
                            state.loading_entry = false;
                            state.entry_key = entry_key.to_string();
                            state.entry_value = entry_value;
                            state.entry_revision = entry["revision"].as_u64();
                            state.entry_operation = entry["operation"].as_str().map(str::to_owned);
                            state.entry_created = entry["created"].as_str().map(str::to_owned);
                        }
                    }
                }
                true
            }
            BackendOperation::GetKvHistory => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                let key = data["key"].as_str().unwrap_or("");
                let history = data["history"].as_array().cloned().unwrap_or_default();
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::KvBucket {
                        connection_id: cid,
                        bucket_name,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                        && *bucket_name == bucket
                        && state.selected_key.as_deref() == Some(key)
                    {
                        state.history = history.clone();
                        state.loading_history = false;
                    }
                }
                true
            }
            BackendOperation::PutKvEntry
            | BackendOperation::DeleteKvEntry
            | BackendOperation::PurgeKvEntry => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                let key = data["key"].as_str().unwrap_or("").to_string();
                let is_put = operation == BackendOperation::PutKvEntry;
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                if !bucket.is_empty() {
                    let mut refresh_cancel = None;
                    for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                        if let TabKind::KvBucket {
                            connection_id: cid,
                            bucket_name,
                            state,
                            guard,
                            ..
                        } = tab
                            && *cid == connection_id
                            && *bucket_name == bucket
                        {
                            let new_gen = crate::tabs::next_generation();
                            state.loading_entries = true;
                            state.load_generation = new_gen;
                            state.keys.clear();
                            state.fetched_values.clear();
                            state.value_search_cursor = 0;
                            state.value_search_scanning = 0;
                            state.keys_complete = false;
                            if is_put && !key.is_empty() {
                                state.selected_key = Some(key.clone());
                                state.show_history = false;
                            }
                            if state.selected_key.as_deref() == Some(key.as_str()) {
                                state.loading_history = true;
                            }
                            refresh_cancel = Some((guard.cancellation(), new_gen));
                        }
                    }
                    if let Some((cancel, generation)) = refresh_cancel {
                        self.backend.send(BackendCommand::ListKvKeys {
                            connection_id,
                            bucket: bucket.clone(),
                            cancel,
                            generation,
                        });
                    }
                    if !key.is_empty() {
                        self.backend.send(BackendCommand::GetKvEntry {
                            connection_id,
                            bucket: bucket.clone(),
                            key: key.clone(),
                        });
                        self.backend.send(BackendCommand::GetKvHistory {
                            connection_id,
                            bucket,
                            key,
                        });
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub(crate) fn clear_kv_loading_on_error(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
    ) {
        match operation {
            BackendOperation::ListKvKeys => {
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::KvBucket {
                        connection_id: tab_cid,
                        state,
                        ..
                    } = tab
                        && *tab_cid == connection_id
                    {
                        state.loading_entries = false;
                    }
                }
            }
            BackendOperation::GetKvHistory => {
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::KvBucket {
                        connection_id: tab_cid,
                        state,
                        ..
                    } = tab
                        && *tab_cid == connection_id
                    {
                        state.loading_history = false;
                    }
                }
            }
            BackendOperation::GetKvEntry => {
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::KvBucket {
                        connection_id: tab_cid,
                        state,
                        ..
                    } = tab
                        && *tab_cid == connection_id
                    {
                        state.loading_entry = false;
                    }
                }
            }
            _ => {}
        }
    }
}
