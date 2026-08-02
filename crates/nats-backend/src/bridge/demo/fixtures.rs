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
pub(super) const ORDERS_STREAM: &str = "ORDERS";
pub(super) const AUDIT_STREAM: &str = "AUDIT_LOG";
pub(super) const TELEMETRY_STREAM: &str = "TELEMETRY";
pub(super) const APP_CONFIG_BUCKET: &str = "app_config";
pub(super) const SERVICE_REGISTRY_BUCKET: &str = "service_registry";
pub(super) const KV_REVISION: u64 = 14;

pub(super) fn system_time() -> SystemTime {
    let since_epoch = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .unwrap_or_default();
    SystemTime::UNIX_EPOCH + since_epoch
}

pub(super) fn streams() -> Vec<StreamInfo> {
    stream_messages()
        .into_iter()
        .map(|(name, messages)| {
            let consumer_count = match name.as_str() {
                ORDERS_STREAM => 2,
                AUDIT_STREAM | TELEMETRY_STREAM => 1,
                _ => 0,
            };
            StreamInfo {
                subjects: vec![
                    match name.as_str() {
                        ORDERS_STREAM => "orders.>",
                        AUDIT_STREAM => "audit.>",
                        TELEMETRY_STREAM => "telemetry.>",
                        _ => "demo.>",
                    }
                    .into(),
                ],
                storage: "Memory".into(),
                retention: "Limits".into(),
                messages: messages.len() as u64,
                bytes: messages
                    .iter()
                    .map(|message| message.payload.len() as u64)
                    .sum(),
                first_sequence: messages.first().map_or(0, |message| message.sequence),
                last_sequence: messages.last().map_or(0, |message| message.sequence),
                consumer_count,
                name,
            }
        })
        .collect()
}

pub(super) fn stream_messages() -> Vec<(String, Vec<StreamMessageInfo>)> {
    vec![
        (
            ORDERS_STREAM.into(),
            json_messages(&[
                (
                    "orders.created",
                    r#"{"order_id":"ord-1001","customer":"Acme","total":149.50,"status":"created"}"#,
                ),
                (
                    "orders.paid",
                    r#"{"order_id":"ord-1001","payment_id":"pay-7001","status":"paid"}"#,
                ),
                (
                    "orders.shipped",
                    r#"{"order_id":"ord-1001","carrier":"DHL","status":"shipped"}"#,
                ),
                (
                    "orders.created",
                    r#"{"order_id":"ord-1002","customer":"Northwind","total":82.00,"status":"created"}"#,
                ),
                (
                    "orders.paid",
                    r#"{"order_id":"ord-1002","payment_id":"pay-7002","status":"paid"}"#,
                ),
                (
                    "orders.cancelled",
                    r#"{"order_id":"ord-1003","reason":"customer_request","status":"cancelled"}"#,
                ),
                (
                    "orders.created",
                    r#"{"order_id":"ord-1004","customer":"Contoso","total":318.75,"status":"created"}"#,
                ),
                (
                    "orders.fulfilled",
                    r#"{"order_id":"ord-1004","warehouse":"eu-central","status":"fulfilled"}"#,
                ),
                (
                    "orders.paid",
                    r#"{"order_id":"ord-1004","payment_id":"pay-7004","status":"paid"}"#,
                ),
                (
                    "orders.shipped",
                    r#"{"order_id":"ord-1004","carrier":"UPS","status":"shipped"}"#,
                ),
                (
                    "orders.created",
                    r#"{"order_id":"ord-1005","customer":"Globex","total":54.25,"status":"created"}"#,
                ),
                (
                    "orders.completed",
                    r#"{"order_id":"ord-1005","status":"completed"}"#,
                ),
            ]),
        ),
        (
            AUDIT_STREAM.into(),
            vec![
                json_message(
                    1,
                    "audit.order.created",
                    r#"{"order_id":"ord-1001","actor":"orders-api","action":"created"}"#,
                ),
                json_message(
                    2,
                    "audit.order.paid",
                    r#"{"order_id":"ord-1001","actor":"payments-worker","action":"paid"}"#,
                ),
                StreamMessageInfo {
                    sequence: 3,
                    subject: "audit.user.login".into(),
                    payload: b"login ok: demo-user".to_vec(),
                    headers: vec![("content-type".into(), "text/plain".into())],
                    time: DEMO_TIME.into(),
                },
                json_message(
                    4,
                    "audit.order.cancelled",
                    r#"{"order_id":"ord-1003","actor":"customer","action":"cancelled"}"#,
                ),
                json_message(
                    5,
                    "audit.config.changed",
                    r#"{"actor":"ops-console","key":"feature.checkout","action":"updated"}"#,
                ),
                json_message(
                    6,
                    "audit.order.completed",
                    r#"{"order_id":"ord-1005","actor":"fulfillment-worker","action":"completed"}"#,
                ),
            ],
        ),
        (
            TELEMETRY_STREAM.into(),
            json_messages(&[
                (
                    "telemetry.orders-api.latency",
                    r#"{"service":"orders-api","metric":"latency_ms","value":42,"status":"ok"}"#,
                ),
                (
                    "telemetry.orders-api.requests",
                    r#"{"service":"orders-api","metric":"requests","value":1280,"status":"ok"}"#,
                ),
                (
                    "telemetry.payments-worker.latency",
                    r#"{"service":"payments-worker","metric":"latency_ms","value":67,"status":"ok"}"#,
                ),
                (
                    "telemetry.audit-writer.queue",
                    r#"{"service":"audit-writer","metric":"queue_depth","value":3,"status":"ok"}"#,
                ),
                (
                    "telemetry.orders-api.errors",
                    r#"{"service":"orders-api","metric":"errors","value":2,"status":"warning"}"#,
                ),
                (
                    "telemetry.gateway.requests",
                    r#"{"service":"gateway","metric":"requests","value":2310,"status":"ok"}"#,
                ),
            ]),
        ),
    ]
}

fn json_messages(items: &[(&str, &str)]) -> Vec<StreamMessageInfo> {
    items
        .iter()
        .enumerate()
        .map(|(index, (subject, payload))| json_message(index as u64 + 1, subject, payload))
        .collect()
}

fn json_message(sequence: u64, subject: &str, payload: &str) -> StreamMessageInfo {
    StreamMessageInfo {
        sequence,
        subject: subject.into(),
        payload: payload.as_bytes().to_vec(),
        headers: vec![("content-type".into(), "application/json".into())],
        time: DEMO_TIME.into(),
    }
}

pub(super) fn consumers() -> Vec<(String, Vec<ConsumerInfo>)> {
    vec![
        (
            ORDERS_STREAM.into(),
            vec![
                consumer(ConsumerSpec {
                    name: "orders-dashboard",
                    stream_name: ORDERS_STREAM,
                    filter_subject: Some("orders.>"),
                    deliver_policy: ConsumerDeliverPolicyKind::All,
                    ack_policy: ConsumerAckPolicyKind::Explicit,
                    description: "Order operations dashboard",
                    num_pending: 8,
                    num_waiting: 1,
                }),
                consumer(ConsumerSpec {
                    name: "orders-fulfillment",
                    stream_name: ORDERS_STREAM,
                    filter_subject: Some("orders.created"),
                    deliver_policy: ConsumerDeliverPolicyKind::Last,
                    ack_policy: ConsumerAckPolicyKind::Explicit,
                    description: "Fulfillment worker",
                    num_pending: 3,
                    num_waiting: 0,
                }),
            ],
        ),
        (
            AUDIT_STREAM.into(),
            vec![consumer(ConsumerSpec {
                name: "audit-archive",
                stream_name: AUDIT_STREAM,
                filter_subject: Some("audit.>"),
                deliver_policy: ConsumerDeliverPolicyKind::All,
                ack_policy: ConsumerAckPolicyKind::All,
                description: "Audit archive writer",
                num_pending: 6,
                num_waiting: 1,
            })],
        ),
        (
            TELEMETRY_STREAM.into(),
            vec![consumer(ConsumerSpec {
                name: "telemetry-monitor",
                stream_name: TELEMETRY_STREAM,
                filter_subject: Some("telemetry.>"),
                deliver_policy: ConsumerDeliverPolicyKind::LastPerSubject,
                ack_policy: ConsumerAckPolicyKind::None,
                description: "Service telemetry monitor",
                num_pending: 2,
                num_waiting: 2,
            })],
        ),
    ]
}

struct ConsumerSpec<'a> {
    name: &'a str,
    stream_name: &'a str,
    filter_subject: Option<&'a str>,
    deliver_policy: ConsumerDeliverPolicyKind,
    ack_policy: ConsumerAckPolicyKind,
    description: &'a str,
    num_pending: u64,
    num_waiting: u64,
}

fn consumer(spec: ConsumerSpec<'_>) -> ConsumerInfo {
    ConsumerInfo {
        name: spec.name.into(),
        stream_name: spec.stream_name.into(),
        durable_name: Some(spec.name.into()),
        filter_subject: spec.filter_subject.map(str::to_owned),
        deliver_policy: spec.deliver_policy,
        ack_policy: ack_label(spec.ack_policy),
        max_deliver: 3,
        max_ack_pending: 1_000,
        description: Some(spec.description.into()),
        deliver_subject: None,
        num_pending: spec.num_pending,
        num_ack_pending: 0,
        num_waiting: spec.num_waiting,
        num_redelivered: 0,
        push_bound: false,
    }
}

pub(super) fn kv_buckets() -> Vec<KvBucketInfo> {
    vec![
        kv_bucket(
            APP_CONFIG_BUCKET,
            "Application feature and order configuration",
            9,
            256,
        ),
        kv_bucket(
            SERVICE_REGISTRY_BUCKET,
            "Service endpoints used by the demo platform",
            5,
            192,
        ),
    ]
}

fn kv_bucket(
    bucket: &str,
    description: &str,
    stored_history_values: u64,
    bytes: u64,
) -> KvBucketInfo {
    KvBucketInfo {
        bucket: bucket.into(),
        stored_history_values,
        history_depth: 5,
        max_age_secs: 0,
        max_age_nanos: 0,
        description: description.into(),
        storage: "Memory".into(),
        bytes,
        max_bytes: -1,
        max_value_size: -1,
        num_replicas: 1,
    }
}

type KvBucketEntries = (String, Vec<(String, Vec<KvHistoryItem>)>);

pub(super) fn kv_entries() -> Vec<KvBucketEntries> {
    vec![
        (
            APP_CONFIG_BUCKET.into(),
            vec![
                (
                    "feature.checkout".into(),
                    history(
                        "feature.checkout",
                        &[
                            (1, r#"{"enabled":false,"rollout":10}"#),
                            (2, r#"{"enabled":true,"rollout":50}"#),
                            (3, r#"{"enabled":true,"rollout":75}"#),
                        ],
                    ),
                ),
                (
                    "feature.search".into(),
                    history(
                        "feature.search",
                        &[(4, r#"{"enabled":false}"#), (5, r#"{"enabled":true}"#)],
                    ),
                ),
                (
                    "service.region".into(),
                    history("service.region", &[(6, r#""eu-west""#)]),
                ),
                (
                    "ui.theme".into(),
                    history("ui.theme", &[(7, r#""system""#)]),
                ),
                (
                    "orders.currency".into(),
                    history("orders.currency", &[(8, r#""USD""#)]),
                ),
                (
                    "orders.max_retries".into(),
                    history("orders.max_retries", &[(9, "3")]),
                ),
            ],
        ),
        (
            SERVICE_REGISTRY_BUCKET.into(),
            vec![
                (
                    "orders.api.url".into(),
                    history(
                        "orders.api.url",
                        &[
                            (10, r#""https://orders.internal/v1""#),
                            (11, r#""https://orders.internal/v2""#),
                        ],
                    ),
                ),
                (
                    "orders.api.version".into(),
                    history("orders.api.version", &[(12, r#""2.4.0""#)]),
                ),
                (
                    "payments.api.url".into(),
                    history(
                        "payments.api.url",
                        &[(13, r#""https://payments.internal/v1""#)],
                    ),
                ),
                (
                    "audit.writer.url".into(),
                    history(
                        "audit.writer.url",
                        &[(14, r#""https://audit.internal/v1""#)],
                    ),
                ),
            ],
        ),
    ]
}

fn history(key: &str, values: &[(u64, &str)]) -> Vec<KvHistoryItem> {
    values
        .iter()
        .map(|(revision, value)| KvHistoryItem {
            key: key.into(),
            value: value.as_bytes().to_vec(),
            revision: *revision,
            delta: 0,
            created: DEMO_TIME.into(),
            operation: "Put".into(),
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
        memory: 12_288,
        storage: 0,
        streams: 3,
        consumers: 4,
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
        api_total: 256,
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
            connections: 5,
            total_connections: 11,
            subscriptions: 10,
            slow_consumers: 0,
            in_msgs: 4_212 + event_count,
            out_msgs: 4_205 + event_count,
            in_bytes: 622_144,
            out_bytes: 619_520,
        }),
        connz: Some(ConnzMetrics {
            open_connections: 5,
            total_connections: 6,
        }),
        jsz: Some(JetStreamMetrics {
            memory_bytes: 12_288,
            storage_bytes: 0,
            streams: 3,
            consumers: 4,
            total_messages: 24 + event_count,
            total_message_bytes: 4_096 + event_count * 112,
            api_total: 256,
            api_errors: 0,
        }),
        errors: Vec::new(),
    }
}

pub(super) fn clients() -> Vec<ClientStatusRow> {
    vec![
        client(ClientSpec {
            client_id: 101,
            state: ClientConnectionState::Open,
            name: "easy-nats-web",
            ip: "127.0.0.1",
            port: 51_010,
            subjects: &["orders.>", "_INBOX.>"],
            closed_at: None,
            closed_reason: None,
        }),
        client(ClientSpec {
            client_id: 102,
            state: ClientConnectionState::Open,
            name: "orders-api",
            ip: "10.0.0.12",
            port: 42_221,
            subjects: &["orders.created", "orders.paid", "orders.completed"],
            closed_at: None,
            closed_reason: None,
        }),
        client(ClientSpec {
            client_id: 103,
            state: ClientConnectionState::Open,
            name: "payments-worker",
            ip: "10.0.0.15",
            port: 42_224,
            subjects: &["payments.>", "orders.paid"],
            closed_at: None,
            closed_reason: None,
        }),
        client(ClientSpec {
            client_id: 104,
            state: ClientConnectionState::Open,
            name: "audit-writer",
            ip: "10.0.0.18",
            port: 42_228,
            subjects: &["audit.>"],
            closed_at: None,
            closed_reason: None,
        }),
        client(ClientSpec {
            client_id: 105,
            state: ClientConnectionState::Open,
            name: "telemetry-agent",
            ip: "10.0.0.21",
            port: 42_230,
            subjects: &["telemetry.>", "_INBOX.>"],
            closed_at: None,
            closed_reason: None,
        }),
        client(ClientSpec {
            client_id: 106,
            state: ClientConnectionState::Closed,
            name: "ops-console",
            ip: "10.0.0.30",
            port: 42_240,
            subjects: &[],
            closed_at: Some("2026-07-25T07:50:00Z"),
            closed_reason: Some("client closed connection"),
        }),
    ]
}

struct ClientSpec<'a> {
    client_id: u64,
    state: ClientConnectionState,
    name: &'a str,
    ip: &'a str,
    port: u16,
    subjects: &'a [&'a str],
    closed_at: Option<&'a str>,
    closed_reason: Option<&'a str>,
}

fn client(spec: ClientSpec<'_>) -> ClientStatusRow {
    ClientStatusRow {
        client_id: spec.client_id,
        state: spec.state,
        name: Some(spec.name.into()),
        account: Some("$G".into()),
        user: Some("demo".into()),
        ip: Some(spec.ip.into()),
        port: Some(spec.port),
        uptime: Some("2h14m".into()),
        idle: Some("0s".into()),
        last_activity: Some(DEMO_TIME.into()),
        rtt: Some("0.8ms".into()),
        subscriptions: Some(spec.subjects.len() as u64),
        pending_bytes: Some(0),
        in_msgs: Some(1_024),
        out_msgs: Some(998),
        in_bytes: Some(131_072),
        out_bytes: Some(126_976),
        language: Some("rust".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        closed_at: spec.closed_at.map(str::to_owned),
        closed_reason: spec.closed_reason.map(str::to_owned),
        subscription_details: spec
            .subjects
            .iter()
            .map(|subject| ClientStatusSubscription {
                subject: (*subject).into(),
            })
            .collect(),
    }
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
