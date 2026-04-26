mod buckets;
mod entries;

use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation};

use super::state::WorkerState;

pub(crate) use buckets::{
    handle_create_bucket, handle_delete_bucket, handle_list_buckets, handle_update_bucket,
};
pub(crate) use entries::{
    handle_delete_entry, handle_get_entry, handle_get_history, handle_list_keys,
    handle_purge_entry, handle_put_entry,
};

async fn jetstream(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
    operation: BackendOperation,
) -> Option<async_nats::jetstream::Context> {
    match state.clients.get(&connection_id) {
        Some(client) => Some(async_nats::jetstream::new(client.clone())),
        None => {
            send_not_connected(evt_tx, connection_id, operation).await;
            None
        }
    }
}

async fn open_store(
    state: &WorkerState,
    connection_id: u64,
    bucket: &str,
    evt_tx: &mpsc::Sender<BackendEvent>,
    operation: BackendOperation,
) -> Option<async_nats::jetstream::kv::Store> {
    open_store_with_error_data(state, connection_id, bucket, evt_tx, operation, None).await
}

async fn open_store_with_error_data(
    state: &WorkerState,
    connection_id: u64,
    bucket: &str,
    evt_tx: &mpsc::Sender<BackendEvent>,
    operation: BackendOperation,
    error_data: Option<serde_json::Value>,
) -> Option<async_nats::jetstream::kv::Store> {
    let js = jetstream(state, connection_id, evt_tx, operation).await?;
    match js.get_key_value(bucket).await {
        Ok(store) => Some(store),
        Err(e) => {
            send_err_with_data(evt_tx, connection_id, operation, e.to_string(), error_data).await;
            None
        }
    }
}

async fn send_ok(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
    data: serde_json::Value,
) {
    tracing::debug!(connection_id, ?operation, "KV operation succeeded");
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
    message: String,
) {
    send_err_with_data(evt_tx, connection_id, operation, message, None).await;
}

async fn send_err_with_data(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
    message: String,
    data: Option<serde_json::Value>,
) {
    tracing::warn!(connection_id, ?operation, %message, "KV operation failed");
    let _ = evt_tx
        .send(BackendEvent::Error {
            connection_id: Some(connection_id),
            backend_id: None,
            operation,
            message,
            data,
        })
        .await;
}

async fn send_not_connected(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
) {
    send_err(
        evt_tx,
        connection_id,
        operation,
        "Not connected".to_string(),
    )
    .await;
}
