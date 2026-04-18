use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation};

use super::state::WorkerState;

pub(crate) async fn handle_get_server_info(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(client) = state.clients.get(&connection_id) else {
        send_err(
            evt_tx,
            connection_id,
            BackendOperation::ServerInfo,
            "Not connected",
        )
        .await;
        return;
    };

    let info = client.server_info();
    let data = serde_json::json!({
        "server_id": info.server_id,
        "server_name": info.server_name,
        "version": info.version,
        "host": info.host,
        "port": info.port,
        "proto": info.proto,
        "go": info.go,
        "max_payload": info.max_payload,
        "client_id": info.client_id,
        "auth_required": info.auth_required,
        "tls_required": info.tls_required,
        "connect_urls": info.connect_urls,
    });
    send_ok(evt_tx, connection_id, BackendOperation::ServerInfo, data).await;
}

pub(crate) async fn handle_get_jetstream_account_info(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(client) = state.clients.get(&connection_id) else {
        send_err(
            evt_tx,
            connection_id,
            BackendOperation::JetStreamAccountInfo,
            "Not connected",
        )
        .await;
        return;
    };

    let js = async_nats::jetstream::new(client.clone());
    match js.query_account().await {
        Ok(account) => {
            let data = serde_json::json!({
                "memory": account.memory,
                "storage": account.storage,
                "streams": account.streams,
                "consumers": account.consumers,
                "domain": account.domain,
                "limits": {
                    "max_memory": account.limits.max_memory,
                    "max_storage": account.limits.max_storage,
                    "max_streams": account.limits.max_streams,
                    "max_consumers": account.limits.max_consumers,
                    "max_ack_pending": account.limits.max_ack_pending,
                    "memory_max_stream_bytes": account.limits.memory_max_stream_bytes,
                    "storage_max_stream_bytes": account.limits.storage_max_stream_bytes,
                    "max_bytes_required": account.limits.max_bytes_required,
                },
                "api_total": account.requests.total,
                "api_errors": account.requests.errors,
            });
            send_ok(
                evt_tx,
                connection_id,
                BackendOperation::JetStreamAccountInfo,
                data,
            )
            .await;
        }
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::JetStreamAccountInfo,
                &e.to_string(),
            )
            .await
        }
    }
}

async fn send_ok(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
    data: serde_json::Value,
) {
    let _ = evt_tx
        .send(BackendEvent::OperationResult {
            connection_id,
            operation,
            data,
        })
        .await;
}

async fn send_err(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
    message: &str,
) {
    let _ = evt_tx
        .send(BackendEvent::Error {
            connection_id: Some(connection_id),
            backend_id: None,
            operation,
            message: message.to_string(),
        })
        .await;
}
