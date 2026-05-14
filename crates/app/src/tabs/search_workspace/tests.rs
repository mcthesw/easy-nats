use nats_backend::StreamMessageInfo;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::tabs::{NormalizedSearchQuery, StreamState, TabGuard};

fn stream_source_and_tab(
    stream_name: &str,
    generation: u64,
    messages: Vec<(&str, &str)>,
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
                    payload: payload.as_bytes().to_vec(),
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
