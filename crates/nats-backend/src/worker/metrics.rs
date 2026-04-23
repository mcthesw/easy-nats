use std::time::SystemTime;

use reqwest::Url;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::event::BackendEvent;
use crate::monitoring::{
    ConnzMetrics, JetStreamMetrics, MetricsHealth, MetricsSection, MetricsSectionError,
    MetricsSnapshot, VarzMetrics,
};

use super::state::WorkerState;

pub(crate) async fn handle_fetch_metrics(
    state: &WorkerState,
    connection_id: u64,
    endpoint: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let endpoint = endpoint.trim().trim_end_matches('/').to_string();
    let mut snapshot = MetricsSnapshot {
        endpoint: endpoint.clone(),
        collected_at: SystemTime::now(),
        health: None,
        varz: None,
        connz: None,
        jsz: None,
        errors: Vec::new(),
    };

    let Ok(base_url) = Url::parse(&endpoint) else {
        snapshot.errors.push(MetricsSectionError {
            section: MetricsSection::Health,
            message: "Invalid monitoring endpoint URL".to_string(),
        });
        send_snapshot(evt_tx, connection_id, snapshot).await;
        return;
    };

    let client = &state.http_client;
    let (health, varz, connz, jsz) = tokio::join!(
        fetch_health(client, base_url.clone()),
        fetch_varz(client, base_url.clone()),
        fetch_connz(client, base_url.clone()),
        fetch_jsz(client, base_url),
    );

    apply_section(
        &mut snapshot.health,
        &mut snapshot.errors,
        MetricsSection::Health,
        health,
    );
    apply_section(
        &mut snapshot.varz,
        &mut snapshot.errors,
        MetricsSection::Varz,
        varz,
    );
    apply_section(
        &mut snapshot.connz,
        &mut snapshot.errors,
        MetricsSection::Connz,
        connz,
    );
    apply_section(
        &mut snapshot.jsz,
        &mut snapshot.errors,
        MetricsSection::Jsz,
        jsz,
    );

    send_snapshot(evt_tx, connection_id, snapshot).await;
}

fn apply_section<T>(
    target: &mut Option<T>,
    errors: &mut Vec<MetricsSectionError>,
    section: MetricsSection,
    result: Result<T, String>,
) {
    match result {
        Ok(value) => *target = Some(value),
        Err(message) => errors.push(MetricsSectionError { section, message }),
    }
}

async fn send_snapshot(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    snapshot: MetricsSnapshot,
) {
    let _ = evt_tx
        .send(BackendEvent::MetricsSnapshot {
            connection_id,
            snapshot: Box::new(snapshot),
        })
        .await;
}

async fn fetch_health(client: &reqwest::Client, base_url: Url) -> Result<MetricsHealth, String> {
    let url = join_url(base_url, "healthz")?;
    let raw = fetch_json::<HealthzResponse>(client, url).await?;
    let status = raw.status.unwrap_or_else(|| "unknown".to_string());
    Ok(MetricsHealth {
        ok: status.eq_ignore_ascii_case("ok"),
        status,
    })
}

async fn fetch_varz(client: &reqwest::Client, base_url: Url) -> Result<VarzMetrics, String> {
    let url = join_url(base_url, "varz")?;
    let raw = fetch_json::<VarzResponse>(client, url).await?;
    Ok(VarzMetrics {
        server_name: raw.server_name,
        server_id: raw.server_id,
        version: raw.version,
        host: raw.host,
        port: raw.port,
        uptime: raw.uptime,
        mem_bytes: raw.mem.unwrap_or_default(),
        cpu_percent: raw.cpu.unwrap_or_default(),
        connections: raw.connections.unwrap_or_default(),
        total_connections: raw.total_connections.unwrap_or_default(),
        subscriptions: raw.subscriptions.unwrap_or_default(),
        slow_consumers: raw.slow_consumers.unwrap_or_default(),
        in_msgs: raw.in_msgs.unwrap_or_default(),
        out_msgs: raw.out_msgs.unwrap_or_default(),
        in_bytes: raw.in_bytes.unwrap_or_default(),
        out_bytes: raw.out_bytes.unwrap_or_default(),
    })
}

async fn fetch_connz(client: &reqwest::Client, base_url: Url) -> Result<ConnzMetrics, String> {
    let mut url = join_url(base_url, "connz")?;
    url.query_pairs_mut().append_pair("limit", "1");
    let raw = fetch_json::<ConnzResponse>(client, url).await?;
    let open_connections = raw.num_connections.or(raw.total).unwrap_or_default();
    let total_connections = raw.total.unwrap_or(open_connections);
    Ok(ConnzMetrics {
        open_connections,
        total_connections,
    })
}

async fn fetch_jsz(client: &reqwest::Client, base_url: Url) -> Result<JetStreamMetrics, String> {
    let url = join_url(base_url, "jsz")?;
    let raw = fetch_json::<JszResponse>(client, url).await?;
    Ok(JetStreamMetrics {
        memory_bytes: raw.memory.unwrap_or_default(),
        storage_bytes: raw.storage.unwrap_or_default(),
        streams: raw.total_streams.unwrap_or_default(),
        consumers: raw.total_consumers.unwrap_or_default(),
        total_messages: raw.total_messages.unwrap_or_default(),
        total_message_bytes: raw.total_message_bytes.unwrap_or_default(),
        api_total: raw
            .api
            .as_ref()
            .and_then(|api| api.total)
            .unwrap_or_default(),
        api_errors: raw
            .api
            .as_ref()
            .and_then(|api| api.errors)
            .unwrap_or_default(),
    })
}

async fn fetch_json<T>(client: &reqwest::Client, url: Url) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|error| format!("{} request failed: {error}", url.path()))?;
    let response = response
        .error_for_status()
        .map_err(|error| format!("{} returned an error: {error}", url.path()))?;
    response
        .json::<T>()
        .await
        .map_err(|error| format!("{} returned invalid JSON: {error}", url.path()))
}

fn join_url(mut base_url: Url, path: &str) -> Result<Url, String> {
    let mut joined = base_url
        .path_segments_mut()
        .map_err(|_| "Monitoring endpoint cannot be a base URL".to_string())?;
    joined.clear().push(path);
    drop(joined);
    Ok(base_url)
}

#[derive(Debug, Deserialize)]
struct HealthzResponse {
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VarzResponse {
    server_name: Option<String>,
    server_id: Option<String>,
    version: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    uptime: Option<String>,
    mem: Option<u64>,
    cpu: Option<f64>,
    connections: Option<u64>,
    total_connections: Option<u64>,
    subscriptions: Option<u64>,
    slow_consumers: Option<u64>,
    in_msgs: Option<u64>,
    out_msgs: Option<u64>,
    in_bytes: Option<u64>,
    out_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ConnzResponse {
    num_connections: Option<u64>,
    total: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct JszResponse {
    memory: Option<u64>,
    storage: Option<u64>,
    total_streams: Option<u64>,
    total_consumers: Option<u64>,
    total_messages: Option<u64>,
    total_message_bytes: Option<u64>,
    api: Option<JszApi>,
}

#[derive(Debug, Deserialize)]
struct JszApi {
    total: Option<u64>,
    errors: Option<u64>,
}
