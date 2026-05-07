use std::time::SystemTime;

use reqwest::Url;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::event::BackendEvent;
use crate::monitoring::{
    ClientConnectionState, ClientStatusDetail, ClientStatusPage, ClientStatusQuery,
    ClientStatusRequestError, ClientStatusRow, ClientStatusSubscription, ConnzMetrics,
    JetStreamMetrics, MetricsHealth, MetricsSection, MetricsSectionError, MetricsSnapshot,
    VarzMetrics,
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
    let url = build_connz_summary_url(base_url)?;
    let raw = fetch_json::<ConnzResponse>(client, url).await?;
    let open_connections = raw.num_connections.or(raw.total).unwrap_or_default();
    let total_connections = raw.total.unwrap_or(open_connections);
    Ok(ConnzMetrics {
        open_connections,
        total_connections,
    })
}

pub(crate) async fn handle_fetch_client_status_page(
    state: &WorkerState,
    connection_id: u64,
    endpoint: String,
    query: ClientStatusQuery,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    tracing::info!(
        connection_id,
        state = ?query.state,
        sort = ?query.sort,
        limit = query.page_size,
        offset = query.offset,
        "Fetching client status page"
    );
    let endpoint = endpoint.trim().trim_end_matches('/').to_string();
    let base_url = match Url::parse(&endpoint) {
        Ok(url) => url,
        Err(_) => {
            send_client_status_error(
                evt_tx,
                connection_id,
                endpoint,
                query,
                "Invalid monitoring endpoint URL".to_string(),
            )
            .await;
            return;
        }
    };

    let url = match build_client_status_page_url(base_url, &query) {
        Ok(url) => url,
        Err(message) => {
            send_client_status_error(evt_tx, connection_id, endpoint, query, message).await;
            return;
        }
    };

    match fetch_json::<ConnzResponse>(&state.http_client, url).await {
        Ok(raw) => {
            let page = normalize_client_status_page(endpoint, SystemTime::now(), query, raw);
            let _ = evt_tx
                .send(BackendEvent::ClientStatusPageLoaded {
                    connection_id,
                    page: Box::new(page),
                })
                .await;
        }
        Err(message) => {
            send_client_status_error(evt_tx, connection_id, endpoint, query, message).await;
        }
    }
}

pub(crate) async fn handle_fetch_client_status_detail(
    state: &WorkerState,
    connection_id: u64,
    endpoint: String,
    query: ClientStatusQuery,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    tracing::info!(
        connection_id,
        client_id = query.client_id,
        include_subscriptions = query.include_subscriptions,
        "Fetching client status detail"
    );
    let endpoint = endpoint.trim().trim_end_matches('/').to_string();
    let base_url = match Url::parse(&endpoint) {
        Ok(url) => url,
        Err(_) => {
            send_client_status_error(
                evt_tx,
                connection_id,
                endpoint,
                query,
                "Invalid monitoring endpoint URL".to_string(),
            )
            .await;
            return;
        }
    };

    let url = match build_client_status_detail_url(base_url, &query) {
        Ok(url) => url,
        Err(message) => {
            send_client_status_error(evt_tx, connection_id, endpoint, query, message).await;
            return;
        }
    };

    match fetch_json::<ConnzResponse>(&state.http_client, url).await {
        Ok(raw) => {
            match normalize_client_status_detail(
                endpoint.clone(),
                SystemTime::now(),
                query.clone(),
                raw,
            ) {
                Ok(detail) => {
                    let _ = evt_tx
                        .send(BackendEvent::ClientStatusDetailLoaded {
                            connection_id,
                            detail: Box::new(detail),
                        })
                        .await;
                }
                Err(message) => {
                    send_client_status_error(evt_tx, connection_id, endpoint, query, message).await;
                }
            }
        }
        Err(message) => {
            send_client_status_error(evt_tx, connection_id, endpoint, query, message).await;
        }
    }
}

async fn send_client_status_error(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    endpoint: String,
    query: ClientStatusQuery,
    message: String,
) {
    tracing::warn!(
        connection_id,
        state = ?query.state,
        sort = ?query.sort,
        limit = query.page_size,
        offset = query.offset,
        has_client_id = query.client_id.is_some(),
        %message,
        "Client status request failed"
    );
    let _ = evt_tx
        .send(BackendEvent::ClientStatusError {
            connection_id,
            error: Box::new(ClientStatusRequestError {
                endpoint,
                collected_at: SystemTime::now(),
                query,
                message,
            }),
        })
        .await;
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

fn build_connz_summary_url(base_url: Url) -> Result<Url, String> {
    let mut url = join_url(base_url, "connz")?;
    url.query_pairs_mut().append_pair("limit", "1");
    Ok(url)
}

fn build_client_status_page_url(base_url: Url, query: &ClientStatusQuery) -> Result<Url, String> {
    let mut url = join_url(base_url, "connz")?;
    let limit = query.page_size.clamp(1, ClientStatusQuery::MAX_PAGE_SIZE);
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("limit", &limit.to_string());
        pairs.append_pair("offset", &query.offset.to_string());
        pairs.append_pair("sort", query.sort.as_connz_param());
        pairs.append_pair("state", query.state.as_connz_param());
        if query.include_auth {
            pairs.append_pair("auth", "1");
        }
    }
    Ok(url)
}

fn build_client_status_detail_url(base_url: Url, query: &ClientStatusQuery) -> Result<Url, String> {
    let Some(client_id) = query.client_id else {
        return Err("Client detail request requires a client ID".to_string());
    };
    let mut url = join_url(base_url, "connz")?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("cid", &client_id.to_string());
        pairs.append_pair("state", query.state.as_connz_param());
        if query.include_subscriptions {
            pairs.append_pair("subs", "1");
        }
        if query.include_auth {
            pairs.append_pair("auth", "1");
        }
    }
    Ok(url)
}

fn normalize_client_status_page(
    endpoint: String,
    collected_at: SystemTime,
    query: ClientStatusQuery,
    raw: ConnzResponse,
) -> ClientStatusPage {
    let total = raw.total.unwrap_or(raw.num_connections.unwrap_or_default());
    let offset = raw.offset.unwrap_or(query.offset);
    let limit = raw.limit.unwrap_or(query.page_size);
    let clients = raw
        .connections
        .into_iter()
        .map(|client| normalize_client_status_row(client, query.state))
        .collect();

    ClientStatusPage {
        endpoint,
        collected_at,
        query,
        total,
        offset,
        limit,
        clients,
    }
}

fn normalize_client_status_detail(
    endpoint: String,
    collected_at: SystemTime,
    query: ClientStatusQuery,
    raw: ConnzResponse,
) -> Result<ClientStatusDetail, String> {
    let client = raw
        .connections
        .into_iter()
        .next()
        .map(|client| normalize_client_status_row(client, query.state))
        .ok_or_else(|| "Client was not found in monitoring response".to_string())?;
    Ok(ClientStatusDetail {
        endpoint,
        collected_at,
        query,
        client,
    })
}

fn normalize_client_status_row(
    raw: ConnzConnection,
    query_state: ClientConnectionState,
) -> ClientStatusRow {
    let state = match query_state {
        ClientConnectionState::Any if raw.stop.is_some() || raw.reason.is_some() => {
            ClientConnectionState::Closed
        }
        ClientConnectionState::Any => ClientConnectionState::Open,
        other => other,
    };
    let subscription_details = raw
        .subscriptions_list
        .into_iter()
        .map(|subject| ClientStatusSubscription { subject })
        .collect();
    ClientStatusRow {
        client_id: raw.cid,
        state,
        name: raw.name,
        account: raw.account,
        user: raw.user.or(raw.username).or(raw.authorized_user),
        ip: raw.ip,
        port: raw.port,
        uptime: raw.uptime,
        idle: raw.idle,
        last_activity: raw.last_activity.or(raw.last),
        rtt: raw.rtt,
        subscriptions: raw.subscriptions.or(raw.num_subscriptions),
        pending_bytes: raw.pending_bytes,
        in_msgs: raw.in_msgs,
        out_msgs: raw.out_msgs,
        in_bytes: raw.in_bytes,
        out_bytes: raw.out_bytes,
        language: raw.lang.or(raw.language),
        version: raw.version,
        closed_at: raw.stop,
        closed_reason: raw.reason,
        subscription_details,
    }
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
    offset: Option<usize>,
    limit: Option<usize>,
    #[serde(default)]
    connections: Vec<ConnzConnection>,
}

#[derive(Debug, Deserialize)]
struct ConnzConnection {
    #[serde(default)]
    cid: u64,
    name: Option<String>,
    account: Option<String>,
    user: Option<String>,
    username: Option<String>,
    authorized_user: Option<String>,
    ip: Option<String>,
    port: Option<u16>,
    uptime: Option<String>,
    idle: Option<String>,
    last_activity: Option<String>,
    last: Option<String>,
    rtt: Option<String>,
    subscriptions: Option<u64>,
    num_subscriptions: Option<u64>,
    pending_bytes: Option<u64>,
    in_msgs: Option<u64>,
    out_msgs: Option<u64>,
    in_bytes: Option<u64>,
    out_bytes: Option<u64>,
    lang: Option<String>,
    language: Option<String>,
    version: Option<String>,
    stop: Option<String>,
    reason: Option<String>,
    #[serde(default, alias = "subs")]
    subscriptions_list: Vec<String>,
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

#[cfg(test)]
mod tests;
