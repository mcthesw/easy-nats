use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::helpers::consumer_info_to_json;
use super::state::WorkerState;

pub(crate) async fn handle_list_consumers(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let lookup_name = stream_name.clone();
    with_stream(
        state,
        connection_id,
        &lookup_name,
        evt_tx,
        "list_consumers",
        |stream| async move {
            let mut consumers_iter = stream.consumers();
            let mut list = Vec::new();
            while let Some(result) = consumers_iter.next().await {
                match result {
                    Ok(info) => list.push(consumer_info_to_json(&info)),
                    Err(e) => {
                        tracing::warn!(%e, "Error iterating consumers");
                        break;
                    }
                }
            }
            send_ok(
                evt_tx,
                connection_id,
                "list_consumers",
                serde_json::json!({
                    "stream": stream_name,
                    "consumers": list,
                }),
            );
        },
    )
    .await;
}

pub(crate) async fn handle_create_consumer(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    config: serde_json::Value,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    with_stream(
        state,
        connection_id,
        &stream_name,
        evt_tx,
        "create_consumer",
        |stream| async move {
            match serde_json::from_value::<async_nats::jetstream::consumer::pull::Config>(config) {
                Ok(consumer_config) => match stream.create_consumer(consumer_config).await {
                    Ok(consumer) => send_ok(
                        evt_tx,
                        connection_id,
                        "create_consumer",
                        consumer_info_to_json(consumer.cached_info()),
                    ),
                    Err(e) => send_err(evt_tx, connection_id, "create_consumer", e.to_string()),
                },
                Err(e) => send_err(
                    evt_tx,
                    connection_id,
                    "create_consumer",
                    format!("Invalid consumer config: {e}"),
                ),
            }
        },
    )
    .await;
}

pub(crate) async fn handle_delete_consumer(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    name: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let lookup_name = stream_name.clone();
    with_stream(
        state,
        connection_id,
        &lookup_name,
        evt_tx,
        "delete_consumer",
        |stream| async move {
            match stream.delete_consumer(&name).await {
                Ok(_) => send_ok(
                    evt_tx,
                    connection_id,
                    "delete_consumer",
                    serde_json::json!({
                        "stream": stream_name,
                        "name": name,
                    }),
                ),
                Err(e) => send_err(evt_tx, connection_id, "delete_consumer", e.to_string()),
            }
        },
    )
    .await;
}

async fn with_stream<F, Fut>(
    state: &WorkerState,
    connection_id: u64,
    stream_name: &str,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
    operation: &str,
    f: F,
) where
    F: FnOnce(async_nats::jetstream::stream::Stream) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    if let Some(client) = state.clients.get(&connection_id) {
        let js = async_nats::jetstream::new(client.clone());
        match js.get_stream(stream_name).await {
            Ok(stream) => f(stream).await,
            Err(e) => send_err(evt_tx, connection_id, operation, e.to_string()),
        }
    } else {
        send_err(
            evt_tx,
            connection_id,
            operation,
            "Not connected".to_string(),
        );
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
