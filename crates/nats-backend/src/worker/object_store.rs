use tokio::sync::mpsc;

use crate::event::BackendEvent;

pub(crate) fn handle_list_buckets(
    connection_id: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    send_err(
        evt_tx,
        connection_id,
        "list_object_store_buckets",
        "Not yet implemented",
    );
}

pub(crate) fn handle_create_bucket(
    connection_id: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    send_err(
        evt_tx,
        connection_id,
        "create_object_store_bucket",
        "Not yet implemented",
    );
}

pub(crate) fn handle_delete_bucket(
    connection_id: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    send_err(
        evt_tx,
        connection_id,
        "delete_object_store_bucket",
        "Not yet implemented",
    );
}

pub(crate) fn handle_query(connection_id: u64, evt_tx: &mpsc::UnboundedSender<BackendEvent>) {
    send_err(
        evt_tx,
        connection_id,
        "object_store_query",
        "Not yet implemented",
    );
}

pub(crate) fn handle_delete_object(
    connection_id: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    send_err(
        evt_tx,
        connection_id,
        "delete_object",
        "Not yet implemented",
    );
}

pub(crate) fn handle_upload_object(
    connection_id: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    send_err(
        evt_tx,
        connection_id,
        "upload_object",
        "Not yet implemented",
    );
}

fn send_err(
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
    connection_id: u64,
    operation: &str,
    message: &str,
) {
    let _ = evt_tx.send(BackendEvent::Error {
        connection_id: Some(connection_id),
        operation: operation.to_string(),
        message: message.to_string(),
    });
}
