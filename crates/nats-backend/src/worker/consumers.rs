use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::helpers::{consumer_info_to_json, raw_message_to_json};
use super::state::WorkerState;

static INSPECTOR_COUNTER: AtomicU64 = AtomicU64::new(0);

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

pub(crate) async fn handle_update_consumer(
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
        "update_consumer",
        |stream| async move {
            match serde_json::from_value::<async_nats::jetstream::consumer::pull::Config>(config) {
                Ok(consumer_config) => match stream.update_consumer(consumer_config).await {
                    Ok(consumer) => send_ok(
                        evt_tx,
                        connection_id,
                        "update_consumer",
                        consumer_info_to_json(consumer.cached_info()),
                    ),
                    Err(e) => send_err(evt_tx, connection_id, "update_consumer", e.to_string()),
                },
                Err(e) => send_err(
                    evt_tx,
                    connection_id,
                    "update_consumer",
                    format!("Invalid consumer config: {e}"),
                ),
            }
        },
    )
    .await;
}

pub(crate) async fn handle_fetch_consumer_messages(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    consumer_name: String,
    batch: usize,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let operation = "fetch_consumer_messages";
    let lookup = stream_name.clone();
    with_stream(
        state,
        connection_id,
        &lookup,
        evt_tx,
        operation,
        |stream| async move {
            // Get the original consumer info to read its filter subjects
            let original = match stream.consumer_info(&consumer_name).await {
                Ok(info) => info,
                Err(e) => {
                    send_err(evt_tx, connection_id, operation, e.to_string());
                    return;
                }
            };

            // Build an ephemeral inspector consumer mirroring the filter
            let filter = original.config.filter_subject.clone();
            let filters = original.config.filter_subjects.clone();
            let unique_id = INSPECTOR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let inspector_config = async_nats::jetstream::consumer::pull::Config {
                name: Some(format!("_easynats_inspect_{unique_id}")),
                filter_subject: filter,
                filter_subjects: filters,
                deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
                memory_storage: true,
                inactive_threshold: std::time::Duration::from_secs(10),
                ..Default::default()
            };

            let inspector = match stream.create_consumer(inspector_config).await {
                Ok(c) => c,
                Err(e) => {
                    send_err(
                        evt_tx,
                        connection_id,
                        operation,
                        format!("Failed to create inspector consumer: {e}"),
                    );
                    return;
                }
            };
            let inspector_name = inspector.cached_info().name.clone();

            let mut fetch_stream = match inspector
                .fetch()
                .max_messages(batch)
                .expires(std::time::Duration::from_secs(5))
                .messages()
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    cleanup_inspector_consumer(&stream, &inspector_name).await;
                    send_err(evt_tx, connection_id, operation, e.to_string());
                    return;
                }
            };

            let mut messages = Vec::new();
            while let Some(result) = fetch_stream.next().await {
                let msg = match result {
                    Ok(msg) => msg,
                    Err(e) => {
                        drop(fetch_stream);
                        cleanup_inspector_consumer(&stream, &inspector_name).await;
                        send_err(evt_tx, connection_id, operation, e.to_string());
                        return;
                    }
                };
                match async_nats::jetstream::message::StreamMessage::try_from(msg.message.clone()) {
                    Ok(stream_msg) => messages.push(raw_message_to_json(&stream_msg)),
                    Err(e) => tracing::debug!(%e, "Failed to convert inspector message"),
                }
                if messages.len() >= batch {
                    break;
                }
            }
            drop(fetch_stream);
            cleanup_inspector_consumer(&stream, &inspector_name).await;

            send_ok(
                evt_tx,
                connection_id,
                operation,
                serde_json::json!({
                    "stream": stream_name,
                    "consumer": consumer_name,
                    "messages": messages,
                }),
            );
        },
    )
    .await;
}

async fn cleanup_inspector_consumer(
    stream: &async_nats::jetstream::stream::Stream,
    consumer_name: &str,
) {
    if let Err(e) = stream.delete_consumer(consumer_name).await {
        tracing::warn!(
            %e,
            consumer_name,
            "Failed to delete inspector consumer"
        );
    }
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
