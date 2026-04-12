mod buckets;
mod entries;

use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::state::WorkerState;

pub(crate) use buckets::{handle_create_bucket, handle_delete_bucket, handle_list_buckets};
pub(crate) use entries::{
    handle_delete_entry, handle_get_entry, handle_get_history, handle_list_keys,
    handle_purge_entry, handle_put_entry,
};

fn jetstream(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
    operation: &str,
) -> Option<async_nats::jetstream::Context> {
    state
        .clients
        .get(&connection_id)
        .map(|client| async_nats::jetstream::new(client.clone()))
        .or_else(|| {
            send_not_connected(evt_tx, connection_id, operation);
            None
        })
}

async fn open_store(
    state: &WorkerState,
    connection_id: u64,
    bucket: &str,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
    operation: &str,
) -> Option<async_nats::jetstream::kv::Store> {
    let js = jetstream(state, connection_id, evt_tx, operation)?;
    match js.get_key_value(bucket).await {
        Ok(store) => Some(store),
        Err(e) => {
            send_err(evt_tx, connection_id, operation, e.to_string());
            None
        }
    }
}

fn send_ok(
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
    connection_id: u64,
    operation: &str,
    data: serde_json::Value,
) {
    let _ = evt_tx.send(BackendEvent::OperationResult {
        connection_id,
        operation: operation.to_string(),
        data,
    });
}

fn send_err(
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
    connection_id: u64,
    operation: &str,
    message: String,
) {
    let _ = evt_tx.send(BackendEvent::Error {
        connection_id: Some(connection_id),
        operation: operation.to_string(),
        message,
    });
}

fn send_not_connected(
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
    connection_id: u64,
    operation: &str,
) {
    send_err(
        evt_tx,
        connection_id,
        operation,
        "Not connected".to_string(),
    );
}
