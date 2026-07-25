use std::time::SystemTime;

use crate::models::{
    ConsumerAckPolicyKind, ConsumerDeliverPolicyKind, ConsumerInfo, JetStreamAccountInfoSnapshot,
    JetStreamAccountLimitsSnapshot, KvBucketInfo, KvHistoryItem, ObjectStoreBucketInfo,
    ObjectStoreObjectInfo, ServerInfoSnapshot, StreamInfo, StreamMessageInfo,
};
use crate::monitoring::{
    ClientConnectionState, ClientStatusRow, ClientStatusSubscription, ConnzMetrics,
    JetStreamMetrics, MetricsHealth, MetricsSnapshot, VarzMetrics,
};

pub(super) const DEMO_CONNECTION_ID: u64 = 1;
pub(super) const DEMO_TIME: &str = "2026-07-25T08:00:00Z";

pub(super) fn system_time() -> SystemTime {
    let since_epoch = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .unwrap_or_default();
    SystemTime::UNIX_EPOCH + since_epoch
}

pub(super) fn stream() -> StreamInfo {
    StreamInfo {
        name: "DEMO_EVENTS".into(),
        subjects: vec!["demo.events.>".into()],
        storage: "Memory".into(),
        retention: "Limits".into(),
        messages: 12,
        bytes: 1_824,
        first_sequence: 1,
        last_sequence: 12,
        consumer_count: 1,
    }
}

pub(super) fn stream_messages() -> Vec<StreamMessageInfo> {
    (8..=12)
        .map(|sequence| StreamMessageInfo {
            sequence,
            subject: if sequence % 2 == 0 {
                "demo.events.orders"
            } else {
                "demo.events.system"
            }
            .into(),
            payload: format!(
                r#"{{"sequence":{sequence},"source":"interactive-demo","status":"ok"}}"#
            )
            .into_bytes(),
            headers: vec![("content-type".into(), "application/json".into())],
            time: DEMO_TIME.into(),
        })
        .collect()
}

pub(super) fn consumer() -> ConsumerInfo {
    ConsumerInfo {
        name: "demo-dashboard".into(),
        stream_name: "DEMO_EVENTS".into(),
        durable_name: Some("demo-dashboard".into()),
        filter_subject: Some("demo.events.>".into()),
        deliver_policy: ConsumerDeliverPolicyKind::All,
        ack_policy: "Explicit".into(),
        max_deliver: 3,
        max_ack_pending: 1_000,
        description: Some("Interactive demo consumer".into()),
        deliver_subject: None,
        num_pending: 4,
        num_ack_pending: 0,
        num_waiting: 1,
        num_redelivered: 0,
        push_bound: false,
    }
}

pub(super) fn kv_bucket() -> KvBucketInfo {
    KvBucketInfo {
        bucket: "demo_config".into(),
        stored_history_values: 3,
        history_depth: 5,
        max_age_secs: 0,
        max_age_nanos: 0,
        description: "Editable demo configuration".into(),
        storage: "Memory".into(),
        bytes: 94,
        max_bytes: -1,
        max_value_size: -1,
        num_replicas: 1,
    }
}

pub(super) fn kv_entries() -> Vec<(String, Vec<KvHistoryItem>)> {
    [
        (
            "feature.checkout",
            br#"{"enabled":true,"rollout":75}"#.to_vec(),
        ),
        ("service.region", br#""eu-west""#.to_vec()),
        ("ui.theme", br#""system""#.to_vec()),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (key, value))| {
        (
            key.into(),
            vec![KvHistoryItem {
                key: key.into(),
                value,
                revision: (index + 1) as u64,
                delta: 0,
                created: DEMO_TIME.into(),
                operation: "Put".into(),
            }],
        )
    })
    .collect()
}

pub(super) fn object_bucket() -> ObjectStoreBucketInfo {
    ObjectStoreBucketInfo {
        bucket: "demo_assets".into(),
        description: "Small in-memory demo objects".into(),
        storage: "Memory".into(),
        bytes: 2_176,
        max_bytes: -1,
        object_count: 2,
        num_replicas: 1,
    }
}

pub(super) fn objects() -> Vec<ObjectStoreObjectInfo> {
    [
        ("welcome.json", 128, "sha-256=demo-welcome"),
        ("sample-report.txt", 2_048, "sha-256=demo-report"),
    ]
    .into_iter()
    .map(|(name, size, digest)| ObjectStoreObjectInfo {
        bucket: "demo_assets".into(),
        name: name.into(),
        description: String::new(),
        size,
        chunks: 1,
        modified: Some(DEMO_TIME.into()),
        digest: Some(digest.into()),
    })
    .collect()
}

pub(super) fn server_info() -> ServerInfoSnapshot {
    ServerInfoSnapshot {
        server_id: "NDEMO000000000000000000000000000000000000000000000000000".into(),
        server_name: "easy-nats-demo".into(),
        version: "2.11-demo".into(),
        host: "127.0.0.1".into(),
        port: 4_222,
        proto: 1,
        go: "go1.24".into(),
        max_payload: 1_048_576,
        client_id: 101,
        auth_required: false,
        tls_required: false,
        connect_urls: vec!["demo.invalid:4222".into()],
    }
}

pub(super) fn account_info() -> JetStreamAccountInfoSnapshot {
    JetStreamAccountInfoSnapshot {
        memory: 4_096,
        storage: 0,
        streams: 1,
        consumers: 1,
        domain: Some("interactive-demo".into()),
        limits: JetStreamAccountLimitsSnapshot {
            max_memory: Some(134_217_728),
            max_storage: Some(1_073_741_824),
            max_streams: Some(20),
            max_consumers: Some(100),
            max_ack_pending: 10_000,
            memory_max_stream_bytes: Some(67_108_864),
            storage_max_stream_bytes: Some(536_870_912),
            max_bytes_required: false,
        },
        api_total: 128,
        api_errors: 0,
    }
}

pub(super) fn metrics(endpoint: String, event_count: u64) -> MetricsSnapshot {
    MetricsSnapshot {
        endpoint,
        collected_at: system_time(),
        health: Some(MetricsHealth {
            ok: true,
            status: "ok".into(),
        }),
        varz: Some(VarzMetrics {
            server_name: Some("easy-nats-demo".into()),
            server_id: Some("NDEMO".into()),
            version: Some("2.11-demo".into()),
            host: Some("127.0.0.1".into()),
            port: Some(8_222),
            uptime: Some("2h14m".into()),
            mem_bytes: 24_117_248,
            cpu_percent: 0.7,
            connections: 3,
            total_connections: 7,
            subscriptions: 5,
            slow_consumers: 0,
            in_msgs: 4_212 + event_count,
            out_msgs: 4_205 + event_count,
            in_bytes: 622_144,
            out_bytes: 619_520,
        }),
        connz: Some(ConnzMetrics {
            open_connections: 3,
            total_connections: 7,
        }),
        jsz: Some(JetStreamMetrics {
            memory_bytes: 4_096,
            storage_bytes: 0,
            streams: 1,
            consumers: 1,
            total_messages: 12 + event_count,
            total_message_bytes: 1_824 + event_count * 96,
            api_total: 128,
            api_errors: 0,
        }),
        errors: Vec::new(),
    }
}

pub(super) fn clients() -> Vec<ClientStatusRow> {
    [
        (101, "easy-nats-web", 2, "127.0.0.1", 51_010),
        (102, "orders-worker", 2, "10.0.0.12", 42_221),
        (103, "audit-service", 1, "10.0.0.18", 42_228),
    ]
    .into_iter()
    .map(
        |(client_id, name, subscriptions, ip, port)| ClientStatusRow {
            client_id,
            state: ClientConnectionState::Open,
            name: Some(name.into()),
            account: Some("$G".into()),
            user: Some("demo".into()),
            ip: Some(ip.into()),
            port: Some(port),
            uptime: Some("2h14m".into()),
            idle: Some("0s".into()),
            last_activity: Some(DEMO_TIME.into()),
            rtt: Some("0.8ms".into()),
            subscriptions: Some(subscriptions),
            pending_bytes: Some(0),
            in_msgs: Some(1_024),
            out_msgs: Some(998),
            in_bytes: Some(131_072),
            out_bytes: Some(126_976),
            language: Some("rust".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            closed_at: None,
            closed_reason: None,
            subscription_details: vec![ClientStatusSubscription {
                subject: if client_id == 101 {
                    "demo.events.>".into()
                } else {
                    format!("demo.service.{client_id}")
                },
            }],
        },
    )
    .collect()
}

pub(super) fn storage_label(storage: crate::models::StorageKind) -> String {
    match storage {
        crate::models::StorageKind::File => "File",
        crate::models::StorageKind::Memory => "Memory",
    }
    .into()
}

pub(super) fn retention_label(retention: crate::models::StreamRetentionKind) -> String {
    match retention {
        crate::models::StreamRetentionKind::Limits => "Limits",
        crate::models::StreamRetentionKind::Interest => "Interest",
        crate::models::StreamRetentionKind::WorkQueue => "WorkQueue",
    }
    .into()
}

pub(super) fn ack_label(ack: ConsumerAckPolicyKind) -> String {
    match ack {
        ConsumerAckPolicyKind::Explicit => "Explicit",
        ConsumerAckPolicyKind::All => "All",
        ConsumerAckPolicyKind::None => "None",
    }
    .into()
}
