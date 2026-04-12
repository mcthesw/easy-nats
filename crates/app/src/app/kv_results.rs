use nats_backend::BackendCommand;

use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::{model::EasyNatsApp, util::decode_kv_value};

impl EasyNatsApp {
    pub(crate) fn apply_kv_operation(
        &mut self,
        connection_id: u64,
        operation: &str,
        data: &serde_json::Value,
    ) -> bool {
        match operation {
            "list_kv_buckets" => {
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
            "create_kv_bucket" | "delete_kv_bucket" => {
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                self.backend
                    .send(BackendCommand::ListKvBuckets { connection_id });
                true
            }
            "list_kv_keys" => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                let entries = data["entries"].as_array().cloned().unwrap_or_default();
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
                        state.entries = entries.clone();
                        state.loading_entries = false;
                    }
                }
                true
            }
            "get_kv_entry" => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                let entry = data["entry"].clone();
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
                        if entry.is_null() {
                            state.entry_revision = None;
                            state.entry_operation = None;
                            state.entry_created = None;
                            state.entry_value.clear();
                        } else {
                            state.entry_key = entry["key"].as_str().unwrap_or("").to_string();
                            state.entry_value = decode_kv_value(&entry);
                            state.entry_revision = entry["revision"].as_u64();
                            state.entry_operation = entry["operation"].as_str().map(str::to_owned);
                            state.entry_created = entry["created"].as_str().map(str::to_owned);
                        }
                    }
                }
                true
            }
            "get_kv_history" => {
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
            "put_kv_entry" | "delete_kv_entry" | "purge_kv_entry" => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                let key = data["key"].as_str().unwrap_or("").to_string();
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                if !bucket.is_empty() {
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
                            state.loading_entries = true;
                            if state.selected_key.as_deref() == Some(key.as_str()) {
                                state.loading_history = true;
                            }
                        }
                    }
                    self.backend.send(BackendCommand::ListKvKeys {
                        connection_id,
                        bucket: bucket.clone(),
                    });
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

    pub(crate) fn clear_kv_loading_on_error(&mut self, connection_id: u64, operation: &str) {
        match operation {
            "list_kv_keys" => {
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
            "get_kv_history" => {
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
            _ => {}
        }
    }
}
