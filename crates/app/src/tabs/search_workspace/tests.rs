use super::*;
use crate::tabs::types::{SearchRecordSnapshot, SearchResultLocator};

fn source(id: &str, generation: u64, text: &str) -> SearchSourceSnapshot {
    let source_id = SearchSourceId::Stream {
        connection_id: 1,
        stream_name: id.to_string(),
    };
    source_with_records(source_id, id, generation, vec![("1", text)])
}

fn source_with_records(
    id: SearchSourceId,
    label: &str,
    generation: u64,
    records: Vec<(&str, &str)>,
) -> SearchSourceSnapshot {
    SearchSourceSnapshot {
        id,
        label: label.to_string(),
        generation,
        coverage: SearchSourceCoverage::Stream {
            messages: records.len(),
        },
        records: records
            .into_iter()
            .enumerate()
            .map(|(idx, (item_id, text))| {
                let sequence = idx as u64 + 1;
                SearchRecordSnapshot {
                    field: SearchField::Payload,
                    item_id: item_id.to_string(),
                    item_label: format!("#{sequence}"),
                    text: text.to_string(),
                    snippet: text.to_string(),
                    locator: SearchResultLocator::StreamMessage {
                        connection_id: 1,
                        stream_name: label.to_string(),
                        sequence,
                    },
                }
            })
            .collect(),
    }
}

#[test]
fn workspace_results_match_selected_sources_only() {
    let selected = SearchSourceId::Stream {
        connection_id: 1,
        stream_name: "orders".to_string(),
    };
    let mut state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![selected],
        ..Default::default()
    };
    let sources = [
        source("orders", 1, "balance: 42"),
        source("payments", 1, "balance: 99"),
    ];

    let results = workspace_results(&mut state, &sources);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_label, "orders");
}

#[test]
fn workspace_cache_invalidates_when_source_generation_changes() {
    let source_id = SearchSourceId::Stream {
        connection_id: 1,
        stream_name: "orders".to_string(),
    };
    let mut state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![source_id],
        ..Default::default()
    };
    let first_sources = [source("orders", 1, "balance: 42")];
    assert_eq!(workspace_results(&mut state, &first_sources).len(), 1);

    let second_sources = [source("orders", 2, "no match")];
    assert_eq!(workspace_results(&mut state, &second_sources).len(), 0);
}

#[test]
fn workspace_results_are_capped() {
    let source_id = SearchSourceId::Stream {
        connection_id: 1,
        stream_name: "orders".to_string(),
    };
    let mut state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![source_id.clone()],
        ..Default::default()
    };
    let records = (0..SEARCH_RESULT_LIMIT + 5)
        .map(|idx| (idx.to_string(), "balance"))
        .collect::<Vec<_>>();
    let record_refs = records
        .iter()
        .map(|(id, text)| (id.as_str(), *text))
        .collect::<Vec<_>>();
    let sources = [source_with_records(source_id, "orders", 1, record_refs)];

    let results = workspace_results(&mut state, &sources);

    assert_eq!(results.len(), SEARCH_RESULT_LIMIT);
}

#[test]
fn workspace_cache_invalidates_when_source_disappears() {
    let source_id = SearchSourceId::Stream {
        connection_id: 1,
        stream_name: "orders".to_string(),
    };
    let mut state = SearchWorkspaceState {
        query: "balance".to_string(),
        selected_sources: vec![source_id],
        ..Default::default()
    };
    let sources = [source("orders", 1, "balance: 42")];
    assert_eq!(workspace_results(&mut state, &sources).len(), 1);

    let no_sources = [];
    let results = workspace_results(&mut state, &no_sources);

    assert!(results.is_empty());
}

#[test]
fn compact_text_collapses_and_truncates() {
    assert_eq!(compact_text("a\n b  c", 8), "a b c");
    assert_eq!(compact_text("abcdef", 3), "abc...");
}
