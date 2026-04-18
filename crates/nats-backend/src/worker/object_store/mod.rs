mod buckets;
mod objects;

use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::state::WorkerState;

pub(crate) use buckets::{handle_create_bucket, handle_delete_bucket, handle_list_buckets};
pub(crate) use objects::{
    handle_delete_object, handle_download_object, handle_list_objects, handle_upload_object,
};

async fn jetstream(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
    operation: &str,
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
    operation: &str,
) -> Option<async_nats::jetstream::object_store::ObjectStore> {
    let js = jetstream(state, connection_id, evt_tx, operation).await?;
    match js.get_object_store(bucket).await {
        Ok(store) => Some(store),
        Err(e) => {
            send_err(evt_tx, connection_id, operation, e.to_string()).await;
            None
        }
    }
}

async fn send_ok(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: &str,
    data: serde_json::Value,
) {
    let _ = evt_tx
        .send(BackendEvent::OperationResult {
            connection_id,
            operation: operation.to_string(),
            data,
        })
        .await;
}

async fn send_err(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: &str,
    message: String,
) {
    let _ = evt_tx
        .send(BackendEvent::Error {
            connection_id: Some(connection_id),
            backend_id: None,
            operation: operation.to_string(),
            message,
        })
        .await;
}

async fn send_not_connected(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: &str,
) {
    send_err(
        evt_tx,
        connection_id,
        operation,
        "Not connected".to_string(),
    )
    .await;
}
