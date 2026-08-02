use tokio_util::sync::CancellationToken;

use super::{BackendHandle, subject_matches};
use crate::{
    BackendCommand, BackendEvent, BackendOperation, ClientConnectionState, ClientStatusQuery,
    ClientStatusSort, StorageKind, StreamConfigInput, StreamRetentionKind, TaskCancellation,
};

#[test]
fn subject_matching_supports_nats_wildcards() {
    assert!(subject_matches("orders.>", "orders.created"));
    assert!(subject_matches("orders.*.created", "orders.api.created"));
    assert!(!subject_matches("orders.*.created", "orders.api.updated"));
    assert!(!subject_matches("orders.>", "orders"));
    assert!(!subject_matches("orders", "orders.created"));
}

#[test]
fn publish_is_delivered_to_matching_demo_subscriber() {
    let mut backend = BackendHandle::spawn();
    backend.send(BackendCommand::Subscribe {
        connection_id: 1,
        backend_id: 42,
        subject: "orders.>".into(),
        cancel: TaskCancellation::new(CancellationToken::new()),
    });
    let _ = backend.drain_events();

    backend.send(BackendCommand::Publish {
        connection_id: 1,
        subject: "orders.manual".into(),
        payload: br#"{"hello":"world"}"#.to_vec(),
        headers: None,
    });

    let events = backend.drain_events();
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::MessageBatch {
            backend_id: 42,
            messages,
            ..
        } if messages.first().is_some_and(|message| message.subject == "orders.manual")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::OperationSucceeded {
            operation: BackendOperation::Publish,
            ..
        }
    )));
}

#[test]
fn request_reply_and_resource_fixtures_are_available() {
    let mut backend = BackendHandle::spawn();
    backend.send(BackendCommand::Request {
        connection_id: 1,
        backend_id: 7,
        request_id: 9,
        subject: "orders.lookup".into(),
        payload: b"hello".to_vec(),
        headers: None,
        timeout_ms: 1_000,
    });
    backend.send(BackendCommand::ListStreams { connection_id: 1 });
    backend.send(BackendCommand::ListKvBuckets { connection_id: 1 });
    backend.send(BackendCommand::ListObjectStoreBuckets { connection_id: 1 });

    let events = backend.drain_events();
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::RequestResponse {
            request_id: 9,
            payload,
            ..
        } if String::from_utf8_lossy(payload).contains("interactive-demo")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::StreamsListed { streams, .. }
            if streams.len() == 3
                && streams.iter().any(|stream| stream.name == "ORDERS")
                && streams.iter().any(|stream| stream.name == "AUDIT_LOG")
                && streams.iter().any(|stream| stream.name == "TELEMETRY")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::KvBucketsListed { buckets, .. }
            if buckets.len() == 2
                && buckets.iter().any(|bucket| bucket.bucket == "app_config")
                && buckets
                    .iter()
                    .any(|bucket| bucket.bucket == "service_registry")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::ObjectStoreBucketsListed { buckets, .. }
            if buckets.iter().any(|bucket| bucket.bucket == "demo_assets")
    )));
}

#[test]
fn kv_mutation_is_visible_without_persistence() {
    let mut backend = BackendHandle::spawn();
    backend.send(BackendCommand::PutKvEntry {
        connection_id: 1,
        bucket: "app_config".into(),
        key: "feature.search".into(),
        value: b"enabled".to_vec(),
    });
    backend.send(BackendCommand::GetKvEntry {
        connection_id: 1,
        bucket: "app_config".into(),
        key: "feature.search".into(),
    });

    assert!(backend.drain_events().iter().any(|event| matches!(
        event,
        BackendEvent::KvEntryFetched { entry, .. }
            if entry.key == "feature.search" && entry.value == b"enabled"
    )));
}

#[test]
fn publish_uses_each_streams_configured_subjects() {
    let mut backend = BackendHandle::spawn();
    backend.send(BackendCommand::CreateStream {
        connection_id: 1,
        config: StreamConfigInput {
            name: "CUSTOM".into(),
            subjects: vec!["custom.events.*".into()],
            storage: StorageKind::Memory,
            retention: StreamRetentionKind::Limits,
            max_messages: None,
            max_bytes: None,
            max_age: None,
            num_replicas: None,
            description: None,
        },
    });
    let _ = backend.drain_events();

    backend.send(BackendCommand::Publish {
        connection_id: 1,
        subject: "custom.events.created".into(),
        payload: b"custom".to_vec(),
        headers: None,
    });
    backend.send(BackendCommand::GetStreamMessages {
        connection_id: 1,
        stream: "CUSTOM".into(),
        start_sequence: None,
        subject_filter: None,
        start_time: None,
        batch_size: 10,
    });

    assert!(backend.drain_events().iter().any(|event| matches!(
        event,
        BackendEvent::StreamMessagesFetched {
            stream,
            messages,
            ..
        } if stream == "CUSTOM"
            && messages
                .iter()
                .any(|message| message.subject == "custom.events.created")
    )));
}

#[test]
fn stream_start_time_takes_precedence_over_sequence() {
    let mut backend = BackendHandle::spawn();
    backend.send(BackendCommand::GetStreamMessages {
        connection_id: 1,
        stream: "ORDERS".into(),
        start_sequence: Some(8),
        subject_filter: None,
        start_time: Some("2026-07-25T09:00:00Z".into()),
        batch_size: 10,
    });

    assert!(backend.drain_events().iter().any(|event| matches!(
        event,
        BackendEvent::StreamMessagesFetched { messages, .. } if messages.is_empty()
    )));
}

#[test]
fn client_pages_apply_state_and_sort_before_pagination() {
    let mut backend = BackendHandle::spawn();
    backend.send(BackendCommand::FetchClientStatusPage {
        connection_id: 1,
        endpoint: "demo://monitoring".into(),
        query: ClientStatusQuery {
            state: ClientConnectionState::Any,
            sort: ClientStatusSort::Subscriptions,
            page_size: 2,
            offset: 0,
            ..Default::default()
        },
    });
    backend.send(BackendCommand::FetchClientStatusPage {
        connection_id: 1,
        endpoint: "demo://monitoring".into(),
        query: ClientStatusQuery {
            state: ClientConnectionState::Closed,
            ..Default::default()
        },
    });

    let events = backend.drain_events();
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::ClientStatusPageLoaded { page, .. }
            if page.total == 6
                && page.clients.len() == 2
                && page.clients[0].client_id == 106
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BackendEvent::ClientStatusPageLoaded { page, .. }
            if page.query.state == ClientConnectionState::Closed
                && page.total == 1
                && page.clients.len() == 1
                && page.clients[0].client_id == 106
    )));
}

#[test]
fn demo_backend_exposes_its_next_scheduled_wakeup() {
    assert!(BackendHandle::spawn().next_wakeup().is_some());
}
