use nats_backend::StreamMessageInfo;
use tokio_util::sync::CancellationToken;

use super::super::types::{PreviewFetchState, SearchResultKey, SearchResultLocator};
use super::*;
use crate::format::{self, PayloadFormat};
use crate::tabs::{
    KvBucketState, NormalizedSearchQuery, ReceivedMessage, StreamState, SubscriberState, TabGuard,
};

fn stream_source_and_tab(
    stream_name: &str,
    generation: u64,
    messages: Vec<(&str, &str)>,
) -> (SearchSourceSummary, TabKind) {
    let messages = messages
        .into_iter()
        .map(|(subject, payload)| (subject, payload.as_bytes().to_vec()))
        .collect();
    stream_source_and_tab_bytes(stream_name, generation, messages)
}

fn stream_source_and_tab_bytes(
    stream_name: &str,
    generation: u64,
    messages: Vec<(&str, Vec<u8>)>,
) -> (SearchSourceSummary, TabKind) {
    let tab = TabKind::Stream {
        connection_id: 1,
        connection_name: "local".to_string(),
        stream_name: stream_name.to_string(),
        guard: TabGuard::new_without_id(CancellationToken::new()),
        state: StreamState {
            messages: messages
                .into_iter()
                .enumerate()
                .map(|(idx, (subject, payload))| StreamMessageInfo {
                    sequence: idx as u64 + 1,
                    subject: subject.to_string(),
                    payload,
                    headers: Vec::new(),
                    time: String::new(),
                })
                .collect(),
            search_generation: generation,
            ..Default::default()
        },
    };
    let source = source_summary_from_tab(&tab).expect("stream tab is searchable");
    (source, tab)
}

fn kv_source_and_tab(
    bucket_name: &str,
    generation: u64,
    values: Vec<(&str, String, Vec<u8>)>,
) -> (SearchSourceSummary, TabKind) {
    let mut state = KvBucketState {
        keys: values.iter().map(|(key, _, _)| key.to_string()).collect(),
        search_generation: generation,
        ..Default::default()
    };
    for (key, decoded_value, raw_value) in values {
        state.fetched_values.insert(key.to_string(), decoded_value);
        state.fetched_value_bytes.insert(key.to_string(), raw_value);
    }
    let tab = TabKind::KvBucket {
        connection_id: 1,
        connection_name: "local".to_string(),
        bucket_name: bucket_name.to_string(),
        guard: TabGuard::new_without_id(CancellationToken::new()),
        state,
    };
    let source = source_summary_from_tab(&tab).expect("KV tab is searchable");
    (source, tab)
}

fn workspace_results(
    state: &SearchWorkspaceState,
    source_tabs: &[(SearchSourceSummary, TabKind)],
) -> Vec<SearchWorkspaceResult> {
    let Some(query) = NormalizedSearchQuery::new(&state.query) else {
        return Vec::new();
    };
    let mut results = Vec::new();
    let mut stats = SearchWorkspaceBuildStats::default();
    for source_id in &state.selected_sources {
        let Some((source, tab)) = source_tabs
            .iter()
            .find(|(source, _)| &source.id == source_id)
        else {
            continue;
        };
        stats.source_count += 1;
        append_search_workspace_results(
            source,
            tab,
            &query,
            state.primary,
            state.secondary,
            &mut results,
            &mut stats,
        );
        if stats.capped {
            break;
        }
    }
    results
}

#[test]
fn workspace_results_match_selected_sources_only() {
    let (orders, orders_tab) =
        stream_source_and_tab("orders", 1, vec![("orders.created", "balance: 42")]);
    let (payments, payments_tab) =
        stream_source_and_tab("payments", 1, vec![("payments.updated", "balance: 99")]);
    let state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![orders.id.clone()],
        ..Default::default()
    };
    let sources = [(orders, orders_tab), (payments, payments_tab)];

    let results = workspace_results(&state, &sources);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_label, "orders (local)");
}

#[test]
fn workspace_results_read_latest_source_generation() {
    let (first_source, first_tab) =
        stream_source_and_tab("orders", 1, vec![("orders.created", "balance: 42")]);
    let state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![first_source.id.clone()],
        ..Default::default()
    };
    assert_eq!(
        workspace_results(&state, &[(first_source, first_tab)]).len(),
        1
    );

    let (second_source, second_tab) =
        stream_source_and_tab("orders", 2, vec![("orders.created", "no match")]);
    let results = workspace_results(&state, &[(second_source, second_tab)]);

    assert!(results.is_empty());
}

#[test]
fn workspace_results_are_capped() {
    let messages = (0..SEARCH_RESULT_LIMIT + 5)
        .map(|idx| (format!("orders.{idx}"), "balance".to_string()))
        .collect::<Vec<_>>();
    let message_refs = messages
        .iter()
        .map(|(subject, payload)| (subject.as_str(), payload.as_str()))
        .collect::<Vec<_>>();
    let (source, tab) = stream_source_and_tab("orders", 1, message_refs);
    let state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![source.id.clone()],
        ..Default::default()
    };

    let results = workspace_results(&state, &[(source, tab)]);

    assert_eq!(results.len(), SEARCH_RESULT_LIMIT);
}

#[test]
fn workspace_results_ignore_missing_selected_sources() {
    let missing = SearchSourceId::Stream {
        connection_id: 1,
        stream_name: "orders".to_string(),
    };
    let state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![missing],
        ..Default::default()
    };

    let results = workspace_results(&state, &[]);

    assert!(results.is_empty());
}

#[test]
fn compact_text_collapses_and_truncates() {
    assert_eq!(compact_text("a\n b  c", 8), "a b c");
    assert_eq!(compact_text("abcdef", 3), "abc...");
}

#[test]
fn workspace_payload_preview_keeps_original_bytes_for_formatting() {
    let (source, tab) =
        stream_source_and_tab_bytes("orders", 1, vec![("orders.created", b"abc\xff".to_vec())]);
    let state = SearchWorkspaceState {
        query: "abc".to_string(),
        selected_sources: vec![source.id.clone()],
        ..Default::default()
    };

    let results = workspace_results(&state, &[(source, tab)]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].snippet, "abc\u{fffd}");
    assert_eq!(results[0].preview_bytes, Some(b"abc\xff".to_vec()));
    let hex = format::format_read_only_preview(
        results[0].preview_bytes.as_deref().unwrap_or(&[]),
        PayloadFormat::Hex,
    );
    assert!(hex.text.contains("61 62 63 FF"));
    assert!(!hex.text.contains("EF BF BD"));
}

#[test]
fn workspace_kv_value_preview_keeps_original_bytes_for_formatting() {
    let (source, tab) = kv_source_and_tab(
        "orders",
        1,
        vec![("orders.1", "abc\u{fffd}".to_string(), b"abc\xff".to_vec())],
    );
    let state = SearchWorkspaceState {
        query: "abc".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: false,
        secondary: true,
        ..Default::default()
    };

    let results = workspace_results(&state, &[(source, tab)]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].snippet, "abc\u{fffd}");
    assert_eq!(results[0].preview_bytes, Some(b"abc\xff".to_vec()));
    let hex = format::format_read_only_preview(
        results[0].preview_bytes.as_deref().unwrap_or(&[]),
        PayloadFormat::Hex,
    );
    assert!(hex.text.contains("61 62 63 FF"));
    assert!(!hex.text.contains("EF BF BD"));
}

#[test]
fn workspace_preview_format_defaults_to_auto_and_persists() {
    let mut state = SearchWorkspaceState::default();
    assert_eq!(state.preview_format, PayloadFormat::Auto);

    state.preview_format = PayloadFormat::Json;
    state.selected_preview = Some(SearchWorkspaceResult {
        key: SearchResultKey {
            source_id: SearchSourceId::Stream {
                connection_id: 1,
                stream_name: "orders".to_string(),
            },
            field: SearchField::Payload,
            item_id: "1".to_string(),
        },
        source_label: "orders (local)".to_string(),
        field: SearchField::Payload,
        item_label: "#1 orders.created".to_string(),
        snippet: "{\"balance\":42}".to_string(),
        preview_bytes: Some(br#"{"balance":42}"#.to_vec()),
        locator: SearchResultLocator::StreamMessage {
            connection_id: 1,
            stream_name: "orders".to_string(),
            sequence: 1,
        },
    });

    state.selected_result = None;

    assert_eq!(state.preview_format, PayloadFormat::Json);
}

#[test]
fn active_preview_result_prefers_current_result_and_marks_stale_snapshot() {
    let (source, tab) = stream_source_and_tab("orders", 1, vec![("orders.created", "balance: 42")]);
    let state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![source.id.clone()],
        ..Default::default()
    };
    let results = workspace_results(&state, &[(source, tab)]);
    let snapshot = results[0].clone();

    let (active, stale) = active_preview_result(&snapshot, &results);
    assert!(!stale);
    assert_eq!(active.snippet, "balance: 42");

    let (active, stale) = active_preview_result(&snapshot, &[]);
    assert!(stale);
    assert_eq!(active.snippet, "balance: 42");
}

fn subscriber_source_and_tab(
    backend_id: u64,
    messages: Vec<(&str, &str)>,
) -> (SearchSourceSummary, TabKind) {
    let messages: Vec<(&str, Vec<u8>)> = messages
        .into_iter()
        .map(|(subject, payload)| (subject, payload.as_bytes().to_vec()))
        .collect();
    subscriber_source_and_tab_bytes(backend_id, messages)
}

fn subscriber_source_and_tab_bytes(
    backend_id: u64,
    messages: Vec<(&str, Vec<u8>)>,
) -> (SearchSourceSummary, TabKind) {
    let messages: std::collections::VecDeque<ReceivedMessage> = messages
        .into_iter()
        .enumerate()
        .map(|(idx, (subject, payload))| ReceivedMessage {
            id: idx as u64 + 1,
            subject: subject.to_string(),
            reply: None,
            headers: Vec::new(),
            payload,
            timestamp: std::time::SystemTime::now(),
            reply_state: None,
            reply_draft: None,
        })
        .collect();
    let tab = TabKind::Subscriber {
        connection_id: 1,
        connection_name: "local".to_string(),
        guard: TabGuard::new_without_id(CancellationToken::new()),
        backend_id,
        state: SubscriberState {
            messages,
            ..Default::default()
        },
    };
    let source = source_summary_from_tab(&tab).expect("subscriber tab is searchable");
    (source, tab)
}

#[test]
fn stream_subject_match_preview_shows_payload() {
    let (source, tab) = stream_source_and_tab_bytes(
        "orders",
        1,
        vec![("orders.created", b"balance: 42".to_vec())],
    );
    // Search by primary field (Subject) only
    let state = SearchWorkspaceState {
        query: "orders".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: true,
        secondary: false,
        ..Default::default()
    };

    let results = workspace_results(&state, &[(source, tab)]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].field, SearchField::Subject);
    // Preview should show the message payload, not the subject string
    assert_eq!(results[0].preview_bytes, Some(b"balance: 42".to_vec()));
    assert_ne!(
        results[0].preview_bytes,
        Some(results[0].item_label.as_bytes().to_vec())
    );
}

#[test]
fn stream_subject_match_preview_keeps_original_bytes_for_formatting() {
    let (source, tab) =
        stream_source_and_tab_bytes("orders", 1, vec![("orders.created", b"abc\xff".to_vec())]);
    let state = SearchWorkspaceState {
        query: "orders".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: true,
        secondary: false,
        ..Default::default()
    };

    let results = workspace_results(&state, &[(source, tab)]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].field, SearchField::Subject);
    assert_eq!(results[0].preview_bytes, Some(b"abc\xff".to_vec()));
    let hex = format::format_read_only_preview(
        results[0].preview_bytes.as_deref().unwrap_or(&[]),
        PayloadFormat::Hex,
    );
    assert!(hex.text.contains("61 62 63 FF"));
}

#[test]
fn subscriber_subject_match_preview_shows_payload() {
    let (source, tab) = subscriber_source_and_tab(2, vec![("events.created", "data: hello")]);
    let state = SearchWorkspaceState {
        query: "events".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: true,
        secondary: false,
        ..Default::default()
    };

    let results = workspace_results(&state, &[(source, tab)]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].field, SearchField::Subject);
    // Preview should show the message payload, not the subject string
    assert_eq!(results[0].preview_bytes, Some(b"data: hello".to_vec()));
}

#[test]
fn subscriber_subject_match_preview_keeps_original_bytes_for_formatting() {
    let (source, tab) =
        subscriber_source_and_tab_bytes(2, vec![("events.created", b"abc\xff".to_vec())]);
    let state = SearchWorkspaceState {
        query: "events".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: true,
        secondary: false,
        ..Default::default()
    };

    let results = workspace_results(&state, &[(source, tab)]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].field, SearchField::Subject);
    assert_eq!(results[0].preview_bytes, Some(b"abc\xff".to_vec()));
    let hex = format::format_read_only_preview(
        results[0].preview_bytes.as_deref().unwrap_or(&[]),
        PayloadFormat::Hex,
    );
    assert!(hex.text.contains("61 62 63 FF"));
}

#[test]
fn kv_key_match_preview_shows_value_when_available() {
    let (source, tab) = kv_source_and_tab(
        "orders",
        1,
        vec![("order/1", "some value".to_string(), b"some value".to_vec())],
    );
    let state = SearchWorkspaceState {
        query: "order".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: true,
        secondary: false,
        ..Default::default()
    };

    let results = workspace_results(&state, &[(source, tab)]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].field, SearchField::Key);
    // KV Key match: preview shows the value (from fetched_value_bytes), not the key name
    assert_eq!(results[0].preview_bytes, Some(b"some value".to_vec()));
    assert_ne!(results[0].preview_bytes, Some(b"order/1".to_vec()));
}

#[test]
fn kv_key_match_preview_is_none_when_value_not_fetched() {
    // Key exists in keys list but value hasn't been fetched yet.
    let state = KvBucketState {
        keys: vec!["order/1".to_string()],
        search_generation: 1,
        ..Default::default()
    };
    let tab = TabKind::KvBucket {
        connection_id: 1,
        connection_name: "local".to_string(),
        bucket_name: "orders".to_string(),
        guard: TabGuard::new_without_id(CancellationToken::new()),
        state,
    };
    let source = source_summary_from_tab(&tab).expect("KV tab is searchable");
    let ws_state = SearchWorkspaceState {
        query: "order".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: true,
        secondary: false,
        ..Default::default()
    };

    let results = workspace_results(&ws_state, &[(source, tab)]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].field, SearchField::Key);
    assert_eq!(results[0].preview_bytes, None);
}

#[test]
fn stream_subject_match_tracks_payload_value_bytes() {
    let (source, tab) =
        stream_source_and_tab_bytes("orders", 1, vec![("orders.created", b"hello".to_vec())]);
    let state = SearchWorkspaceState {
        query: "orders".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: true,
        secondary: false,
        ..Default::default()
    };

    let mut results = Vec::new();
    let mut stats = SearchWorkspaceBuildStats::default();
    let query = NormalizedSearchQuery::new(&state.query).expect("normalized query");
    append_search_workspace_results(&source, &tab, &query, true, false, &mut results, &mut stats);

    assert_eq!(results.len(), 1);
    assert_eq!(stats.payload_value_bytes, 5); // "hello" is 5 bytes
}

#[test]
fn preview_fetch_state_default_is_idle() {
    let state = PreviewFetchState::default();
    assert!(matches!(state, PreviewFetchState::Idle));
}

#[test]
fn preview_fetch_state_transition_idle_to_loading_to_idle() {
    let key = SearchResultKey {
        source_id: SearchSourceId::Kv {
            connection_id: 1,
            bucket_name: "orders".to_string(),
        },
        field: SearchField::Key,
        item_id: "order/1".to_string(),
    };

    // Idle -> Loading
    let state = PreviewFetchState::Loading(key.clone());
    assert!(matches!(&state, PreviewFetchState::Loading(k) if k == &key));

    // Loading -> Idle (value arrived)
    let state = PreviewFetchState::Idle;
    assert!(matches!(state, PreviewFetchState::Idle));
}

#[test]
fn preview_fetch_state_transition_idle_to_loading_to_failed() {
    let key = SearchResultKey {
        source_id: SearchSourceId::Kv {
            connection_id: 1,
            bucket_name: "orders".to_string(),
        },
        field: SearchField::Key,
        item_id: "order/1".to_string(),
    };

    // Idle -> Loading -> Failed (fetch error)
    let state = PreviewFetchState::Failed {
        key: key.clone(),
        message: "connection refused".to_string(),
    };
    match &state {
        PreviewFetchState::Failed { key: k, message } => {
            assert_eq!(k, &key);
            assert_eq!(message, "connection refused");
        }
        _ => panic!("expected Failed state"),
    }
}

#[test]
fn preview_fetch_state_transition_failed_to_loading_retry() {
    let key = SearchResultKey {
        source_id: SearchSourceId::Kv {
            connection_id: 1,
            bucket_name: "orders".to_string(),
        },
        field: SearchField::Key,
        item_id: "order/1".to_string(),
    };

    // Failed -> Loading (user clicks retry)
    let _before = PreviewFetchState::Failed {
        key: key.clone(),
        message: "connection refused".to_string(),
    };
    let state = PreviewFetchState::Loading(key.clone());
    assert!(matches!(&state, PreviewFetchState::Loading(k) if k == &key));
}

#[test]
fn search_result_key_equality_ignores_generation() {
    // SearchResultKey deliberately excludes generation from its identity.
    // Two keys with identical source_id, field, and item_id must be equal.
    let key_a = SearchResultKey {
        source_id: SearchSourceId::Kv {
            connection_id: 1,
            bucket_name: "orders".to_string(),
        },
        field: SearchField::Key,
        item_id: "order/1".to_string(),
    };
    let key_b = key_a.clone();
    assert_eq!(key_a, key_b);

    // Different item_id → not equal
    let key_c = SearchResultKey {
        item_id: "order/2".to_string(),
        ..key_a.clone()
    };
    assert_ne!(key_a, key_c);

    // Different field → not equal
    let key_d = SearchResultKey {
        field: SearchField::Value,
        ..key_a.clone()
    };
    assert_ne!(key_a, key_d);

    // Different source_id → not equal
    let key_e = SearchResultKey {
        source_id: SearchSourceId::Kv {
            connection_id: 2,
            bucket_name: "orders".to_string(),
        },
        ..key_a.clone()
    };
    assert_ne!(key_a, key_e);
}

#[test]
fn stale_kv_key_match_with_none_preview_does_not_trigger_fetch() {
    // Simulate: KV Key match selected, value not fetched (preview_bytes = None).
    // Key is then deleted → results rebuild without it → snapshot is stale.
    // active_preview_result must return is_stale = true so render_preview
    // shows "value_not_found" instead of emitting an infinite fetch loop.
    let (source, _tab) = kv_source_and_tab("orders", 1, Vec::new());
    let ws_state = SearchWorkspaceState {
        query: "order".to_string(),
        selected_sources: vec![source.id.clone()],
        primary: true,
        secondary: false,
        ..Default::default()
    };

    // Build results with a key that has no fetched value (preview_bytes = None).
    // We need a key in the keys list but not in fetched_value_bytes.
    let state_with_key = KvBucketState {
        keys: vec!["order/1".to_string()],
        search_generation: 1,
        ..Default::default()
    };
    let tab_with_key = TabKind::KvBucket {
        connection_id: 1,
        connection_name: "local".to_string(),
        bucket_name: "orders".to_string(),
        guard: TabGuard::new_without_id(CancellationToken::new()),
        state: state_with_key,
    };
    let source_with_key = source_summary_from_tab(&tab_with_key).expect("KV tab is searchable");
    let results = workspace_results(&ws_state, &[(source_with_key, tab_with_key)]);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].preview_bytes, None);

    // Snapshot the selected result.
    let snapshot = results[0].clone();

    // Simulate key deletion: rebuild with empty keys (generation bumped).
    let state_deleted = KvBucketState {
        keys: Vec::new(),
        search_generation: 2,
        ..Default::default()
    };
    let tab_deleted = TabKind::KvBucket {
        connection_id: 1,
        connection_name: "local".to_string(),
        bucket_name: "orders".to_string(),
        guard: TabGuard::new_without_id(CancellationToken::new()),
        state: state_deleted,
    };
    let source_deleted = source_summary_from_tab(&tab_deleted).expect("KV tab is searchable");
    let rebuilt_results = workspace_results(&ws_state, &[(source_deleted, tab_deleted)]);

    // The key is gone from rebuilt results.
    assert!(rebuilt_results.iter().all(|r| r.key != snapshot.key));

    // active_preview_result marks the snapshot as stale.
    let (active, is_stale) = active_preview_result(&snapshot, &rebuilt_results);
    assert!(is_stale);
    assert_eq!(active.preview_bytes, None);
    // The render_preview guard: is_stale && preview_bytes == None → show "value_not_found",
    // do NOT emit fetch. This test verifies the precondition for that guard.
}
