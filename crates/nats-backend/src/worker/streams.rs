use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::helpers::{raw_message_to_json, stream_info_to_json};
use super::state::WorkerState;

pub(crate) async fn handle_list_streams(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(js) = jetstream(state, connection_id, evt_tx, "list_streams") else {
        return;
    };

    let mut stream_iter = js.streams();
    let mut list = Vec::new();
    while let Some(result) = stream_iter.next().await {
        match result {
            Ok(info) => list.push(stream_info_to_json(&info)),
            Err(e) => {
                tracing::warn!(%e, "Error iterating streams");
                break;
            }
        }
    }
    send_ok(
        evt_tx,
        connection_id,
        "list_streams",
        serde_json::Value::Array(list),
    );
}

pub(crate) async fn handle_create_stream(
    state: &WorkerState,
    connection_id: u64,
    config: serde_json::Value,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(js) = jetstream(state, connection_id, evt_tx, "create_stream") else {
        return;
    };

    match serde_json::from_value::<async_nats::jetstream::stream::Config>(config) {
        Ok(stream_config) => match js.create_stream(stream_config).await {
            Ok(stream) => send_ok(
                evt_tx,
                connection_id,
                "create_stream",
                stream_info_to_json(stream.cached_info()),
            ),
            Err(e) => send_err(evt_tx, connection_id, "create_stream", e.to_string()),
        },
        Err(e) => send_err(
            evt_tx,
            connection_id,
            "create_stream",
            format!("Invalid stream config: {e}"),
        ),
    }
}

pub(crate) async fn handle_update_stream(
    state: &WorkerState,
    connection_id: u64,
    config: serde_json::Value,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(js) = jetstream(state, connection_id, evt_tx, "update_stream") else {
        return;
    };

    match serde_json::from_value::<async_nats::jetstream::stream::Config>(config) {
        Ok(stream_config) => match js.update_stream(stream_config).await {
            Ok(info) => send_ok(
                evt_tx,
                connection_id,
                "update_stream",
                stream_info_to_json(&info),
            ),
            Err(e) => send_err(evt_tx, connection_id, "update_stream", e.to_string()),
        },
        Err(e) => send_err(
            evt_tx,
            connection_id,
            "update_stream",
            format!("Invalid stream config: {e}"),
        ),
    }
}

pub(crate) async fn handle_delete_stream(
    state: &WorkerState,
    connection_id: u64,
    name: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(js) = jetstream(state, connection_id, evt_tx, "delete_stream") else {
        return;
    };

    match js.delete_stream(&name).await {
        Ok(_) => send_ok(
            evt_tx,
            connection_id,
            "delete_stream",
            serde_json::json!({ "name": name }),
        ),
        Err(e) => send_err(evt_tx, connection_id, "delete_stream", e.to_string()),
    }
}

pub(crate) async fn handle_purge_stream(
    state: &WorkerState,
    connection_id: u64,
    name: String,
    filter: Option<String>,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(stream) = open_stream(state, connection_id, &name, evt_tx, "purge_stream").await
    else {
        return;
    };

    let result = if let Some(filter_subject) = filter {
        stream.purge().filter(filter_subject.as_str()).await
    } else {
        stream.purge().await
    };

    match result {
        Ok(resp) => send_ok(
            evt_tx,
            connection_id,
            "purge_stream",
            serde_json::json!({ "name": name, "purged": resp.purged }),
        ),
        Err(e) => send_err(evt_tx, connection_id, "purge_stream", e.to_string()),
    }
}

pub(crate) async fn handle_get_messages(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    start_sequence: Option<u64>,
    subject_filter: Option<String>,
    batch_size: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(stream) = open_stream(
        state,
        connection_id,
        &stream_name,
        evt_tx,
        "get_stream_messages",
    )
    .await
    else {
        return;
    };

    let info = stream.cached_info();
    let first = start_sequence.unwrap_or(info.state.first_sequence);
    let last = info.state.last_sequence;
    let mut messages = Vec::new();

    if let Some(filter) = subject_filter {
        let mut seq = first;
        while (messages.len() as u64) < batch_size {
            match stream.get_first_raw_message_by_subject(&filter, seq).await {
                Ok(raw) => {
                    seq = raw.sequence + 1;
                    messages.push(raw_message_to_json(&raw));
                }
                Err(_) => break,
            }
        }
    } else {
        let mut seq = first;
        while seq <= last && (messages.len() as u64) < batch_size {
            if let Ok(raw) = stream.get_raw_message(seq).await {
                messages.push(raw_message_to_json(&raw));
            }
            seq += 1;
        }
    }

    send_ok(
        evt_tx,
        connection_id,
        "get_stream_messages",
        serde_json::json!({ "stream": stream_name, "messages": messages }),
    );
}

pub(crate) async fn handle_delete_message(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    sequence: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(stream) = open_stream(
        state,
        connection_id,
        &stream_name,
        evt_tx,
        "delete_stream_message",
    )
    .await
    else {
        return;
    };

    match stream.delete_message(sequence).await {
        Ok(_) => send_ok(
            evt_tx,
            connection_id,
            "delete_stream_message",
            serde_json::json!({ "stream": stream_name, "sequence": sequence }),
        ),
        Err(e) => send_err(
            evt_tx,
            connection_id,
            "delete_stream_message",
            e.to_string(),
        ),
    }
}

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

async fn open_stream(
    state: &WorkerState,
    connection_id: u64,
    stream_name: &str,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
    operation: &str,
) -> Option<async_nats::jetstream::stream::Stream> {
    let js = jetstream(state, connection_id, evt_tx, operation)?;
    match js.get_stream(stream_name).await {
        Ok(stream) => Some(stream),
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
