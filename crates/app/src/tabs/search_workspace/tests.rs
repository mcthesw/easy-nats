use nats_backend::StreamMessageInfo;
use tokio_util::sync::CancellationToken;

use super::super::types::{SearchResultIdentity, SearchResultLocator};
use super::*;
use crate::format::{self, PayloadFormat};
use crate::tabs::{KvBucketState, NormalizedSearchQuery, StreamState, TabGuard};

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
    assert_eq!(results[0].preview_bytes, b"abc\xff");
    let hex = format::format_read_only_preview(&results[0].preview_bytes, PayloadFormat::Hex);
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
    assert_eq!(results[0].preview_bytes, b"abc\xff");
    let hex = format::format_read_only_preview(&results[0].preview_bytes, PayloadFormat::Hex);
    assert!(hex.text.contains("61 62 63 FF"));
    assert!(!hex.text.contains("EF BF BD"));
}

#[test]
fn workspace_preview_format_defaults_to_auto_and_persists() {
    let mut state = SearchWorkspaceState::default();
    assert_eq!(state.preview_format, PayloadFormat::Auto);

    state.preview_format = PayloadFormat::Json;
    state.selected_preview = Some(SearchWorkspaceResult {
        identity: SearchResultIdentity {
            source_id: SearchSourceId::Stream {
                connection_id: 1,
                stream_name: "orders".to_string(),
            },
            generation: 1,
            field: SearchField::Payload,
            item_id: "1".to_string(),
        },
        source_label: "orders (local)".to_string(),
        field: SearchField::Payload,
        item_label: "#1 orders.created".to_string(),
        snippet: "{\"balance\":42}".to_string(),
        preview_bytes: br#"{"balance":42}"#.to_vec(),
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
