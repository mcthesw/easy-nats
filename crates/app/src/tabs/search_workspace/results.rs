use super::compact_text;
use crate::tabs::common::{
    NormalizedSearchQuery, SEARCH_RESULT_LIMIT, format_timestamp, searchable_payload_text,
};
use crate::tabs::types::{
    KvBucketState, ReceivedMessage, SearchField, SearchResultIdentity, SearchResultLocator,
    SearchSourceKind, SearchSourceSummary, SearchWorkspaceResult, StreamState, SubscriberState,
    TabKind,
};

#[derive(Debug, Default)]
pub(crate) struct SearchWorkspaceBuildStats {
    pub source_count: usize,
    pub records_scanned: usize,
    pub payload_value_bytes: usize,
    pub capped: bool,
}

struct ResultBuildContext<'a, 'r> {
    source: &'a SearchSourceSummary,
    query: &'a NormalizedSearchQuery,
    primary: bool,
    secondary: bool,
    results: &'r mut Vec<SearchWorkspaceResult>,
    stats: &'r mut SearchWorkspaceBuildStats,
}

fn result_limit_reached(ctx: &mut ResultBuildContext<'_, '_>) -> bool {
    if ctx.results.len() < SEARCH_RESULT_LIMIT {
        return false;
    }
    ctx.stats.capped = true;
    true
}

pub(crate) fn append_search_workspace_results(
    source: &SearchSourceSummary,
    tab: &TabKind,
    query: &NormalizedSearchQuery,
    primary: bool,
    secondary: bool,
    results: &mut Vec<SearchWorkspaceResult>,
    stats: &mut SearchWorkspaceBuildStats,
) {
    if results.len() >= SEARCH_RESULT_LIMIT {
        stats.capped = true;
        return;
    }

    let ctx = ResultBuildContext {
        source,
        query,
        primary,
        secondary,
        results,
        stats,
    };

    match (source.kind, tab) {
        (
            SearchSourceKind::Kv,
            TabKind::KvBucket {
                connection_id,
                bucket_name,
                state,
                ..
            },
        ) => append_kv_results(ctx, *connection_id, bucket_name, state),
        (
            SearchSourceKind::Stream,
            TabKind::Stream {
                connection_id,
                stream_name,
                state,
                ..
            },
        ) => append_stream_results(ctx, *connection_id, stream_name, state),
        (
            SearchSourceKind::Subscriber,
            TabKind::Subscriber {
                connection_id,
                backend_id,
                state,
                ..
            },
        ) => append_subscriber_results(ctx, *connection_id, *backend_id, state),
        _ => {}
    }
}

fn append_kv_results(
    mut ctx: ResultBuildContext<'_, '_>,
    connection_id: u64,
    bucket_name: &str,
    state: &KvBucketState,
) {
    if ctx.primary {
        for key in &state.keys {
            if result_limit_reached(&mut ctx) {
                return;
            }
            ctx.stats.records_scanned += 1;
            if ctx.query.matches(key) {
                push_result(
                    &mut ctx,
                    SearchField::Key,
                    key,
                    key,
                    key,
                    key.as_bytes(),
                    SearchResultLocator::KvKey {
                        connection_id,
                        bucket_name: bucket_name.to_string(),
                        key: key.clone(),
                    },
                );
            }
        }
    }

    if ctx.secondary {
        let mut fetched_values = state.fetched_values.iter().collect::<Vec<_>>();
        fetched_values.sort_by_key(|(key, _)| *key);
        for (key, value) in fetched_values {
            if result_limit_reached(&mut ctx) {
                return;
            }
            ctx.stats.records_scanned += 1;
            ctx.stats.payload_value_bytes += value.len();
            if ctx.query.matches(value) {
                let preview_bytes = state
                    .fetched_value_bytes
                    .get(key.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_else(|| value.as_bytes());
                push_result(
                    &mut ctx,
                    SearchField::Value,
                    key,
                    key,
                    value,
                    preview_bytes,
                    SearchResultLocator::KvKey {
                        connection_id,
                        bucket_name: bucket_name.to_string(),
                        key: key.to_string(),
                    },
                );
            }
        }
    }
}

fn append_stream_results(
    mut ctx: ResultBuildContext<'_, '_>,
    connection_id: u64,
    stream_name: &str,
    state: &StreamState,
) {
    for msg in &state.messages {
        let sequence = msg.sequence;
        let subject = msg.subject.as_str();

        if ctx.primary && !subject.is_empty() {
            if result_limit_reached(&mut ctx) {
                return;
            }
            ctx.stats.records_scanned += 1;
            if ctx.query.matches(subject) {
                let item_label = stream_item_label(sequence, subject);
                push_result(
                    &mut ctx,
                    SearchField::Subject,
                    &sequence.to_string(),
                    &item_label,
                    subject,
                    subject.as_bytes(),
                    SearchResultLocator::StreamMessage {
                        connection_id,
                        stream_name: stream_name.to_string(),
                        sequence,
                    },
                );
            }
        }

        if ctx.secondary {
            if result_limit_reached(&mut ctx) {
                return;
            }
            ctx.stats.records_scanned += 1;
            ctx.stats.payload_value_bytes += msg.payload.len();
            let payload = searchable_payload_text(&msg.payload);
            if !payload.is_empty() && ctx.query.matches(&payload) {
                let item_label = stream_item_label(sequence, subject);
                push_result(
                    &mut ctx,
                    SearchField::Payload,
                    &sequence.to_string(),
                    &item_label,
                    &payload,
                    &msg.payload,
                    SearchResultLocator::StreamMessage {
                        connection_id,
                        stream_name: stream_name.to_string(),
                        sequence,
                    },
                );
            }
        }
    }
}

fn append_subscriber_results(
    mut ctx: ResultBuildContext<'_, '_>,
    connection_id: u64,
    backend_id: u64,
    state: &SubscriberState,
) {
    for msg in &state.messages {
        if ctx.primary {
            if result_limit_reached(&mut ctx) {
                return;
            }
            ctx.stats.records_scanned += 1;
            if ctx.query.matches(&msg.subject) {
                let item_label = subscriber_item_label(msg);
                push_result(
                    &mut ctx,
                    SearchField::Subject,
                    &msg.id.to_string(),
                    &item_label,
                    &msg.subject,
                    msg.subject.as_bytes(),
                    SearchResultLocator::SubscriberMessage {
                        connection_id,
                        backend_id,
                        message_id: msg.id,
                    },
                );
            }
        }

        if ctx.secondary {
            if result_limit_reached(&mut ctx) {
                return;
            }
            ctx.stats.records_scanned += 1;
            ctx.stats.payload_value_bytes += msg.payload.len();
            let payload = searchable_payload_text(&msg.payload);
            if !payload.is_empty() && ctx.query.matches(&payload) {
                let item_label = subscriber_item_label(msg);
                push_result(
                    &mut ctx,
                    SearchField::Payload,
                    &msg.id.to_string(),
                    &item_label,
                    &payload,
                    &msg.payload,
                    SearchResultLocator::SubscriberMessage {
                        connection_id,
                        backend_id,
                        message_id: msg.id,
                    },
                );
            }
        }
    }
}

fn stream_item_label(sequence: u64, subject: &str) -> String {
    if subject.is_empty() {
        format!("#{sequence}")
    } else {
        format!("#{sequence} {subject}")
    }
}

fn subscriber_item_label(msg: &ReceivedMessage) -> String {
    format!("{} {}", format_timestamp(msg.timestamp), msg.subject)
}

fn push_result(
    ctx: &mut ResultBuildContext<'_, '_>,
    field: SearchField,
    item_id: &str,
    item_label: &str,
    text: &str,
    preview_bytes: &[u8],
    locator: SearchResultLocator,
) {
    ctx.results.push(SearchWorkspaceResult {
        identity: SearchResultIdentity {
            source_id: ctx.source.id.clone(),
            generation: ctx.source.generation,
            field,
            item_id: item_id.to_string(),
        },
        source_label: ctx.source.label.clone(),
        field,
        item_label: item_label.to_string(),
        snippet: compact_text(text, 160),
        preview_bytes: preview_bytes.to_vec(),
        locator,
    });
}
