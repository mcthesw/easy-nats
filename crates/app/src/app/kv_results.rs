use nats_backend::{
    BackendCommand, BackendErrorContext, BackendOperation, KvBucketInfo, KvEntryInfo,
    KvHistoryItem, KvKeyBatch,
};

use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_kv_buckets(&mut self, connection_id: u64, buckets: Vec<KvBucketInfo>) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::KvBucket {
                connection_id: cid,
                bucket_name,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.info = buckets
                    .iter()
                    .find(|info| info.bucket == *bucket_name)
                    .cloned();
            }
        }
        self.kv_lists.insert(connection_id, buckets);
    }

    pub(crate) fn apply_kv_bucket_changed(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        bucket: KvBucketInfo,
    ) {
        self.toasts
            .push(ToastLevel::Success, format!("{operation} succeeded"));
        upsert_kv_bucket(
            self.kv_lists.entry(connection_id).or_default(),
            bucket.clone(),
        );
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::KvBucket {
                connection_id: cid,
                bucket_name,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *bucket_name == bucket.bucket
            {
                state.info = Some(bucket.clone());
            }
        }
        self.backend
            .send(BackendCommand::ListKvBuckets { connection_id });
    }

    pub(crate) fn apply_kv_bucket_deleted(&mut self, connection_id: u64, bucket: String) {
        self.toasts.push(
            ToastLevel::Success,
            format!("{} succeeded", BackendOperation::DeleteKvBucket),
        );
        if let Some(buckets) = self.kv_lists.get_mut(&connection_id) {
            buckets.retain(|info| info.bucket != bucket);
        }
        self.remove_tabs_matching(|tab| {
            matches!(tab, TabKind::KvBucket { connection_id: cid, bucket_name, .. }
                if *cid == connection_id && bucket_name == &bucket)
        });
        self.backend
            .send(BackendCommand::ListKvBuckets { connection_id });
    }

    pub(crate) fn apply_kv_key_batch(&mut self, connection_id: u64, batch: KvKeyBatch) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::KvBucket {
                connection_id: cid,
                bucket_name,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *bucket_name == batch.bucket
                && state.load_generation == batch.generation
            {
                state.keys.extend(batch.keys.clone());
                if !batch.keys.is_empty() {
                    state.invalidate_filtered_key_cache();
                    state.search_generation = state.search_generation.wrapping_add(1);
                }
                if batch.done {
                    state.keys.sort();
                    state.invalidate_filtered_key_cache();
                    state.keys_complete = true;
                    state.loading_entries = false;
                    state.search_generation = state.search_generation.wrapping_add(1);
                }
            }
        }
    }

    pub(crate) fn apply_kv_entry(&mut self, connection_id: u64, entry: KvEntryInfo) {
        let entry_key = entry.key.as_str();
        let entry_value = decode_kv_bytes(&entry.value);
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::KvBucket {
                connection_id: cid,
                bucket_name,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *bucket_name == entry.bucket
            {
                state
                    .fetched_values
                    .insert(entry_key.to_string(), entry_value.clone());
                state
                    .fetched_value_bytes
                    .insert(entry_key.to_string(), entry.value.clone());
                state.invalidate_filtered_key_cache();
                state.search_generation = state.search_generation.wrapping_add(1);
                if state.value_search_pending.remove(entry_key) {
                    state.value_search_scanning = state.value_search_pending.len();
                }
                if state.selected_key.as_deref() == Some(entry_key) {
                    state.loading_entry = false;
                    state.entry_key = entry_key.to_string();
                    state.entry_value = entry_value.clone();
                    state.entry_revision = entry.revision;
                    state.entry_operation = entry.operation.clone();
                    state.entry_created = entry.created.clone();
                }
            }
        }
    }

    pub(crate) fn apply_kv_history(
        &mut self,
        connection_id: u64,
        bucket: String,
        key: String,
        history: Vec<KvHistoryItem>,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::KvBucket {
                connection_id: cid,
                bucket_name,
                state,
                ..
            } = tab
                && *cid == connection_id
                && *bucket_name == bucket
                && state.selected_key.as_deref() == Some(key.as_str())
            {
                state.history = history.clone();
                state.invalidate_filtered_key_cache();
                state.search_generation = state.search_generation.wrapping_add(1);
                state.loading_history = false;
            }
        }
    }

    pub(crate) fn apply_kv_entry_mutation(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        bucket: String,
        key: String,
    ) {
        let is_put = operation == BackendOperation::PutKvEntry;
        self.toasts
            .push(ToastLevel::Success, format!("{operation} succeeded"));

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
                state.fetched_value_bytes.clear();
                state.invalidate_filtered_key_cache();
                state.value_search_cursor = 0;
                state.value_search_scanning = 0;
                state.value_search_pending.clear();
                state.search_generation = state.search_generation.wrapping_add(1);
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

    pub(crate) fn clear_kv_loading_on_error(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        context: Option<&BackendErrorContext>,
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
                let (failed_bucket, failed_key) = match context {
                    Some(BackendErrorContext::KvEntry { bucket, key }) => {
                        (Some(bucket.as_str()), Some(key.as_str()))
                    }
                    _ => (None, None),
                };
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::KvBucket {
                        connection_id: tab_cid,
                        bucket_name,
                        state,
                        ..
                    } = tab
                        && *tab_cid == connection_id
                    {
                        let failed_selected_entry = match (failed_bucket, failed_key) {
                            (Some(bucket), Some(key)) => {
                                bucket_name == bucket && state.selected_key.as_deref() == Some(key)
                            }
                            (Some(bucket), None) => bucket_name == bucket,
                            (None, _) => true,
                        };
                        if failed_selected_entry {
                            state.loading_entry = false;
                        }
                        if failed_bucket.is_some_and(|bucket| bucket_name == bucket)
                            && let Some(key) = failed_key
                            && state.value_search_pending.remove(key)
                        {
                            state.value_search_scanning = state.value_search_pending.len();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn upsert_kv_bucket(buckets: &mut Vec<KvBucketInfo>, bucket: KvBucketInfo) {
    if let Some(existing) = buckets.iter_mut().find(|info| info.bucket == bucket.bucket) {
        *existing = bucket;
    } else {
        buckets.push(bucket);
        buckets.sort_by(|a, b| a.bucket.cmp(&b.bucket));
    }
}

fn decode_kv_bytes(value: &[u8]) -> String {
    String::from_utf8_lossy(value).to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use egui_dock::DockState;
    use nats_backend::{BackendErrorContext, BackendOperation, KvEntryInfo};
    use tokio_util::sync::CancellationToken;

    use super::EasyNatsApp;
    use crate::log_layer::LogBuffer;
    use crate::settings::AppSettings;
    use crate::tabs::{KvBucketState, TabGuard, TabKind};
    use crate::theme::ThemeId;

    fn test_app_with_kv_tab(
        connection_id: u64,
        bucket_name: &str,
        state: KvBucketState,
    ) -> EasyNatsApp {
        let mut app = EasyNatsApp::new(
            AppSettings::default(),
            ThemeId::EguiDark,
            Arc::new(Mutex::new(LogBuffer::default())),
        );
        app.dock_state = DockState::new(vec![TabKind::KvBucket {
            connection_id,
            connection_name: "local".to_string(),
            bucket_name: bucket_name.to_string(),
            guard: TabGuard::new_without_id(CancellationToken::new()),
            state,
        }]);
        app
    }

    #[test]
    fn apply_kv_entry_keeps_raw_value_bytes() {
        let mut app = test_app_with_kv_tab(7, "ORDERS", KvBucketState::default());
        app.apply_kv_entry(
            7,
            KvEntryInfo {
                bucket: "ORDERS".to_string(),
                key: "orders/1".to_string(),
                value: b"abc\xff".to_vec(),
                revision: Some(1),
                delta: None,
                created: None,
                operation: None,
            },
        );

        let (_, tab) = app
            .dock_state
            .iter_all_tabs()
            .next()
            .expect("KV tab exists");
        let TabKind::KvBucket { state, .. } = tab else {
            panic!("expected KV bucket tab");
        };
        assert_eq!(
            state.fetched_values.get("orders/1").map(String::as_str),
            Some("abc\u{fffd}")
        );
        assert_eq!(
            state.fetched_value_bytes.get("orders/1").map(Vec::as_slice),
            Some(b"abc\xff".as_slice())
        );
    }

    #[test]
    fn get_kv_entry_error_releases_matching_value_scan_request() {
        let mut state = KvBucketState {
            value_search_scanning: 1,
            ..Default::default()
        };
        state.value_search_pending.insert("orders/1".to_string());
        let mut app = test_app_with_kv_tab(7, "ORDERS", state);
        let context = BackendErrorContext::KvEntry {
            bucket: "ORDERS".to_string(),
            key: "orders/1".to_string(),
        };

        app.clear_kv_loading_on_error(7, BackendOperation::GetKvEntry, Some(&context));

        let (_, tab) = app
            .dock_state
            .iter_all_tabs()
            .next()
            .expect("KV tab exists");
        let TabKind::KvBucket { state, .. } = tab else {
            panic!("expected KV bucket tab");
        };
        assert_eq!(state.value_search_scanning, 0);
        assert!(state.value_search_pending.is_empty());
    }

    #[test]
    fn scan_error_does_not_clear_unrelated_selected_entry_loading_state() {
        let mut state = KvBucketState {
            loading_entry: true,
            selected_key: Some("orders/2".to_string()),
            value_search_scanning: 1,
            ..Default::default()
        };
        state.value_search_pending.insert("orders/1".to_string());
        let mut app = test_app_with_kv_tab(7, "ORDERS", state);
        let context = BackendErrorContext::KvEntry {
            bucket: "ORDERS".to_string(),
            key: "orders/1".to_string(),
        };

        app.clear_kv_loading_on_error(7, BackendOperation::GetKvEntry, Some(&context));

        let (_, tab) = app
            .dock_state
            .iter_all_tabs()
            .next()
            .expect("KV tab exists");
        let TabKind::KvBucket { state, .. } = tab else {
            panic!("expected KV bucket tab");
        };
        assert!(state.loading_entry);
        assert_eq!(state.value_search_scanning, 0);
        assert!(state.value_search_pending.is_empty());
    }
}
