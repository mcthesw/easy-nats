use nats_backend::BackendCommand;

use crate::i18n::t;
use crate::tabs::{
    KV_VALUE_SEARCH_BATCH, MessageSchemasState, NormalizedSearchQuery, SearchResultLocator,
    SearchSourceId, SearchSourceSummary, SearchWorkspaceBuildStats, SearchWorkspaceCacheKey,
    SearchWorkspaceResult, SearchWorkspaceState, TabKind, append_search_workspace_results,
    source_summary_from_tab,
};
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn search_source_summaries(&self) -> Vec<SearchSourceSummary> {
        self.dock_state
            .iter_all_tabs()
            .filter_map(|(_, tab)| source_summary_from_tab(tab))
            .collect()
    }

    pub(crate) fn prepare_search_workspace_results(&mut self, sources: &[SearchSourceSummary]) {
        let requests = self.search_workspace_refresh_requests(sources);
        if requests.is_empty() {
            return;
        }

        let updates = requests
            .into_iter()
            .map(|request| {
                let started = std::time::Instant::now();
                let (results, stats) = self.build_search_workspace_results(
                    &request.key,
                    &request.selected_sources,
                    sources,
                );
                tracing::debug!(
                    workspace_index = request.workspace_index,
                    refresh_reason = request.refresh_reason,
                    source_count = stats.source_count,
                    records_scanned = stats.records_scanned,
                    payload_value_bytes = stats.payload_value_bytes,
                    result_count = results.len(),
                    capped = stats.capped,
                    elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
                    "Refreshed Search Workspace results"
                );
                (request.workspace_index, request.key, results)
            })
            .collect::<Vec<_>>();

        let mut updates = updates.into_iter().peekable();
        let mut workspace_index = 0usize;
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::SearchWorkspace { state } = tab {
                if let Some((_, key, results)) =
                    updates.next_if(|(idx, _, _)| *idx == workspace_index)
                {
                    state.cached_results = Some((key, results));
                }
                workspace_index += 1;
            }
        }
    }

    fn search_workspace_refresh_requests(
        &self,
        sources: &[SearchSourceSummary],
    ) -> Vec<SearchWorkspaceRefreshRequest> {
        let mut requests = Vec::new();
        let mut workspace_index = 0usize;
        for (_surface, tab) in self.dock_state.iter_all_tabs() {
            if let TabKind::SearchWorkspace { state } = tab {
                let (key, refresh_reason) = search_workspace_refresh_decision(state, sources);
                if let Some(refresh_reason) = refresh_reason {
                    requests.push(SearchWorkspaceRefreshRequest {
                        workspace_index,
                        selected_sources: state.selected_sources.clone(),
                        refresh_reason,
                        key,
                    });
                }
                workspace_index += 1;
            }
        }
        requests
    }

    fn build_search_workspace_results(
        &self,
        key: &SearchWorkspaceCacheKey,
        selected_sources: &[SearchSourceId],
        sources: &[SearchSourceSummary],
    ) -> (Vec<SearchWorkspaceResult>, SearchWorkspaceBuildStats) {
        let mut results = Vec::new();
        let mut stats = SearchWorkspaceBuildStats::default();
        if key.query.is_empty() || (!key.primary && !key.secondary) {
            return (results, stats);
        }

        let query =
            NormalizedSearchQuery::new(&key.query).expect("active search has a normalized query");
        for source_id in selected_sources {
            let Some(source) = sources.iter().find(|source| &source.id == source_id) else {
                continue;
            };
            let Some((_surface, tab)) = self
                .dock_state
                .iter_all_tabs()
                .find(|(_, tab)| tab_matches_search_source(tab, source_id))
            else {
                continue;
            };
            stats.source_count += 1;
            append_search_workspace_results(
                source,
                tab,
                &query,
                key.primary,
                key.secondary,
                &mut results,
                &mut stats,
            );
            if stats.capped {
                break;
            }
        }
        (results, stats)
    }

    pub(crate) fn open_or_focus_search_workspace(&mut self) {
        self.open_or_focus_tab_kind(TabKind::SearchWorkspace {
            state: SearchWorkspaceState::default(),
        });
    }

    pub(crate) fn open_or_focus_message_schemas(&mut self) {
        self.open_or_focus_tab_kind(TabKind::MessageSchemas {
            state: MessageSchemasState::default(),
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

    pub(crate) fn fetch_search_workspace_kv_value(
        &mut self,
        source_id: &SearchSourceId,
        key: &str,
    ) {
        let SearchSourceId::Kv {
            connection_id,
            bucket_name,
        } = source_id
        else {
            return;
        };

        let mut send_request = None;
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
                // Double-check before sending: value not already cached,
                // not already being fetched by batch scan, and key still exists.
                if state.fetched_value_bytes.contains_key(key)
                    || state.value_search_pending.contains(key)
                    || !state.keys.iter().any(|k| k == key)
                {
                    break;
                }
                send_request = Some((*connection_id, bucket_name.clone(), key.to_string()));
                break;
            }
        }

        if let Some((cid, bucket, key)) = send_request {
            self.backend.send(BackendCommand::GetKvEntry {
                connection_id: cid,
                bucket,
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
                    .position(|msg| msg.sequence == sequence)
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

#[derive(Debug)]
struct SearchWorkspaceRefreshRequest {
    workspace_index: usize,
    selected_sources: Vec<SearchSourceId>,
    refresh_reason: &'static str,
    key: SearchWorkspaceCacheKey,
}

fn search_workspace_refresh_decision(
    state: &SearchWorkspaceState,
    sources: &[SearchSourceSummary],
) -> (SearchWorkspaceCacheKey, Option<&'static str>) {
    let key = search_workspace_cache_key(state, sources);
    let cached_key = state.cached_results.as_ref().map(|(key, _)| key);
    let refresh_reason = search_workspace_refresh_reason(cached_key, &key);
    (key, refresh_reason)
}

fn search_workspace_refresh_reason(
    cached_key: Option<&SearchWorkspaceCacheKey>,
    next_key: &SearchWorkspaceCacheKey,
) -> Option<&'static str> {
    match cached_key {
        None => Some("uncached"),
        Some(cached_key) if cached_key.query != next_key.query => Some("query"),
        Some(cached_key)
            if cached_key.primary != next_key.primary
                || cached_key.secondary != next_key.secondary =>
        {
            Some("field_scope")
        }
        Some(cached_key) if cached_key.sources != next_key.sources => Some("sources_or_generation"),
        Some(_) => None,
    }
}

fn search_workspace_cache_key(
    state: &SearchWorkspaceState,
    sources: &[SearchSourceSummary],
) -> SearchWorkspaceCacheKey {
    SearchWorkspaceCacheKey {
        query: state.query.trim().to_lowercase(),
        primary: state.primary,
        secondary: state.secondary,
        sources: state
            .selected_sources
            .iter()
            .map(|source_id| {
                let generation = sources
                    .iter()
                    .find(|source| &source.id == source_id)
                    .map(|source| source.generation);
                (source_id.clone(), generation)
            })
            .collect(),
    }
}

fn tab_matches_search_source(tab: &TabKind, source_id: &SearchSourceId) -> bool {
    match (tab, source_id) {
        (
            TabKind::KvBucket {
                connection_id,
                bucket_name,
                ..
            },
            SearchSourceId::Kv {
                connection_id: source_connection_id,
                bucket_name: source_bucket,
            },
        ) => connection_id == source_connection_id && bucket_name == source_bucket,
        (
            TabKind::Stream {
                connection_id,
                stream_name,
                ..
            },
            SearchSourceId::Stream {
                connection_id: source_connection_id,
                stream_name: source_stream,
            },
        ) => connection_id == source_connection_id && stream_name == source_stream,
        (
            TabKind::Subscriber {
                connection_id,
                backend_id,
                ..
            },
            SearchSourceId::Subscriber {
                connection_id: source_connection_id,
                backend_id: source_backend_id,
            },
        ) => connection_id == source_connection_id && backend_id == source_backend_id,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use egui_dock::DockState;
    use nats_backend::{BackendEvent, BackendOperation, StreamMessageInfo};
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::log_layer::LogBuffer;
    use crate::settings::AppSettings;
    use crate::tabs::{KvBucketState, StreamState, TabGuard};
    use crate::theme::ThemeId;

    fn stream_source(stream_name: &str, generation: u64) -> SearchSourceSummary {
        let tab = TabKind::Stream {
            connection_id: 1,
            connection_name: "local".to_string(),
            stream_name: stream_name.to_string(),
            guard: TabGuard::new_without_id(CancellationToken::new()),
            state: StreamState {
                messages: vec![StreamMessageInfo {
                    sequence: 1,
                    subject: "orders.created".to_string(),
                    payload: b"balance: 42".to_vec(),
                    headers: Vec::new(),
                    time: String::new(),
                }],
                search_generation: generation,
                ..Default::default()
            },
        };
        source_summary_from_tab(&tab).expect("stream tab is searchable")
    }

    #[test]
    fn workspace_cache_key_normalizes_query_and_tracks_scope() {
        let source = stream_source("orders", 1);
        let mut state = SearchWorkspaceState {
            query: "  Balance  ".to_string(),
            selected_sources: vec![source.id.clone()],
            ..Default::default()
        };

        let initial = search_workspace_cache_key(&state, std::slice::from_ref(&source));
        assert_eq!(initial.query, "balance");
        assert!(initial.primary);
        assert!(initial.secondary);

        state.primary = false;
        let primary_changed = search_workspace_cache_key(&state, std::slice::from_ref(&source));
        assert_ne!(initial, primary_changed);

        state.secondary = false;
        let secondary_changed = search_workspace_cache_key(&state, std::slice::from_ref(&source));
        assert_ne!(primary_changed, secondary_changed);
    }

    #[test]
    fn workspace_cache_key_tracks_selection_order_and_source_generation() {
        let orders_v1 = stream_source("orders", 1);
        let payments = stream_source("payments", 1);
        let mut state = SearchWorkspaceState {
            query: "balance".to_string(),
            selected_sources: vec![orders_v1.id.clone(), payments.id.clone()],
            ..Default::default()
        };
        let ordered = search_workspace_cache_key(&state, &[orders_v1.clone(), payments.clone()]);

        state.selected_sources.reverse();
        let reversed = search_workspace_cache_key(&state, &[orders_v1.clone(), payments]);
        assert_ne!(ordered, reversed);

        state.selected_sources.reverse();
        let orders_v2 = stream_source("orders", 2);
        let generation_changed = search_workspace_cache_key(&state, &[orders_v2]);
        assert_ne!(ordered, generation_changed);
    }

    #[test]
    fn workspace_cache_key_tracks_disappeared_sources() {
        let source = stream_source("orders", 1);
        let state = SearchWorkspaceState {
            query: "balance".to_string(),
            selected_sources: vec![source.id.clone()],
            ..Default::default()
        };

        let present = search_workspace_cache_key(&state, std::slice::from_ref(&source));
        let missing = search_workspace_cache_key(&state, &[]);

        assert_eq!(missing.sources, vec![(source.id, None)]);
        assert_ne!(present, missing);
    }

    #[test]
    fn workspace_refresh_reuses_cache_until_key_changes() {
        let source = stream_source("orders", 1);
        let mut state = SearchWorkspaceState {
            query: "balance".to_string(),
            selected_sources: vec![source.id.clone()],
            ..Default::default()
        };
        let key = search_workspace_cache_key(&state, std::slice::from_ref(&source));
        state.cached_results = Some((key, Vec::new()));

        let (_, unchanged) =
            search_workspace_refresh_decision(&state, std::slice::from_ref(&source));
        assert!(unchanged.is_none());

        state.query = "other".to_string();
        let (_, query_changed) =
            search_workspace_refresh_decision(&state, std::slice::from_ref(&source));
        assert_eq!(query_changed, Some("query"));
    }

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

    /// Polls the backend event channel for a short period and returns true if
    /// a `GetKvEntry` error event is received.
    fn backend_emitted_get_kv_entry_error(app: &mut EasyNatsApp) -> bool {
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            if let Some(event) = app.backend.try_recv()
                && let BackendEvent::Error { operation, .. } = event
                && operation == BackendOperation::GetKvEntry
            {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn fetch_kv_value_sends_get_kv_entry_when_guards_pass() {
        let state = KvBucketState {
            keys: vec!["order/1".to_string()],
            ..Default::default()
        };
        let mut app = test_app_with_kv_tab(7, "ORDERS", state);
        let source_id = SearchSourceId::Kv {
            connection_id: 7,
            bucket_name: "ORDERS".to_string(),
        };

        app.fetch_search_workspace_kv_value(&source_id, "order/1");

        assert!(
            backend_emitted_get_kv_entry_error(&mut app),
            "expected a GetKvEntry error event after fetching an uncached key"
        );
    }

    #[test]
    fn fetch_kv_value_skips_when_value_already_cached() {
        let mut state = KvBucketState {
            keys: vec!["order/1".to_string()],
            ..Default::default()
        };
        state
            .fetched_value_bytes
            .insert("order/1".to_string(), b"value".to_vec());
        let mut app = test_app_with_kv_tab(7, "ORDERS", state);
        let source_id = SearchSourceId::Kv {
            connection_id: 7,
            bucket_name: "ORDERS".to_string(),
        };

        app.fetch_search_workspace_kv_value(&source_id, "order/1");

        assert!(
            !backend_emitted_get_kv_entry_error(&mut app),
            "should not send GetKvEntry when value is already cached"
        );
    }

    #[test]
    fn fetch_kv_value_skips_when_key_in_value_search_pending() {
        let mut state = KvBucketState {
            keys: vec!["order/1".to_string()],
            value_search_scanning: 1,
            ..Default::default()
        };
        state.value_search_pending.insert("order/1".to_string());
        let mut app = test_app_with_kv_tab(7, "ORDERS", state);
        let source_id = SearchSourceId::Kv {
            connection_id: 7,
            bucket_name: "ORDERS".to_string(),
        };

        app.fetch_search_workspace_kv_value(&source_id, "order/1");

        assert!(
            !backend_emitted_get_kv_entry_error(&mut app),
            "should not send GetKvEntry when key is in value_search_pending"
        );
    }

    #[test]
    fn fetch_kv_value_skips_when_key_not_in_keys_list() {
        let state = KvBucketState {
            keys: vec!["order/2".to_string()],
            ..Default::default()
        };
        let mut app = test_app_with_kv_tab(7, "ORDERS", state);
        let source_id = SearchSourceId::Kv {
            connection_id: 7,
            bucket_name: "ORDERS".to_string(),
        };

        app.fetch_search_workspace_kv_value(&source_id, "order/1");

        assert!(
            !backend_emitted_get_kv_entry_error(&mut app),
            "should not send GetKvEntry when key is not in the keys list"
        );
    }

    #[test]
    fn fetch_kv_value_skips_when_kv_tab_does_not_exist() {
        let mut app = EasyNatsApp::new(
            AppSettings::default(),
            ThemeId::EguiDark,
            Arc::new(Mutex::new(LogBuffer::default())),
        );
        app.dock_state = DockState::new(vec![TabKind::Welcome]);
        let source_id = SearchSourceId::Kv {
            connection_id: 7,
            bucket_name: "ORDERS".to_string(),
        };

        // Should not panic and should not send anything.
        app.fetch_search_workspace_kv_value(&source_id, "order/1");

        assert!(
            !backend_emitted_get_kv_entry_error(&mut app),
            "should not send GetKvEntry when no matching KV tab exists"
        );
    }

    #[test]
    fn fetch_kv_value_ignores_non_kv_source_id() {
        let state = KvBucketState {
            keys: vec!["order/1".to_string()],
            ..Default::default()
        };
        let mut app = test_app_with_kv_tab(7, "ORDERS", state);
        let source_id = SearchSourceId::Stream {
            connection_id: 7,
            stream_name: "ORDERS".to_string(),
        };

        // Should return early for non-Kv source_id without panicking.
        app.fetch_search_workspace_kv_value(&source_id, "order/1");

        assert!(
            !backend_emitted_get_kv_entry_error(&mut app),
            "should not send GetKvEntry for a non-Kv source_id"
        );
    }
}
