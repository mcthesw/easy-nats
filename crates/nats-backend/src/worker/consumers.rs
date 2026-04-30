use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation};
use crate::models::{ConsumerConfigInput, ConsumerInfo, StreamMessageInfo};

use super::state::WorkerState;

static INSPECTOR_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) async fn handle_list_consumers(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let lookup_name = stream_name.clone();
    with_stream(
        state,
        connection_id,
        &lookup_name,
        evt_tx,
        BackendOperation::ListConsumers,
        |stream| async move {
            let mut consumers_iter = stream.consumers();
            let mut list = Vec::new();
            while let Some(result) = consumers_iter.next().await {
                match result {
                    Ok(info) => list.push(ConsumerInfo::from_info(&info)),
                    Err(e) => {
                        tracing::warn!(%e, "Error iterating consumers");
                        break;
                    }
                }
            }
            let _ = evt_tx
                .send(BackendEvent::ConsumersListed {
                    connection_id,
                    stream: stream_name,
                    consumers: list,
                })
                .await;
        },
    )
    .await;
}

pub(crate) async fn handle_create_consumer(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    config: ConsumerConfigInput,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let event_stream_name = stream_name.clone();
    with_stream(
        state,
        connection_id,
        &stream_name,
        evt_tx,
        BackendOperation::CreateConsumer,
        |stream| async move {
            match stream.create_consumer(config.into_async_nats_pull()).await {
                Ok(consumer) => {
                    let _ = evt_tx
                        .send(BackendEvent::ConsumerCreated {
                            connection_id,
                            stream: event_stream_name,
                            consumer: ConsumerInfo::from_info(consumer.cached_info()),
                        })
                        .await;
                }
                Err(e) => {
                    send_err(
                        evt_tx,
                        connection_id,
                        BackendOperation::CreateConsumer,
                        e.to_string(),
                    )
                    .await
                }
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
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let lookup_name = stream_name.clone();
    with_stream(
        state,
        connection_id,
        &lookup_name,
        evt_tx,
        BackendOperation::DeleteConsumer,
        |stream| async move {
            match stream.delete_consumer(&name).await {
                Ok(_) => {
                    let _ = evt_tx
                        .send(BackendEvent::ConsumerDeleted {
                            connection_id,
                            stream: stream_name,
                            name,
                        })
                        .await;
                }
                Err(e) => {
                    send_err(
                        evt_tx,
                        connection_id,
                        BackendOperation::DeleteConsumer,
                        e.to_string(),
                    )
                    .await
                }
            }
        },
    )
    .await;
}

pub(crate) async fn handle_update_consumer(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    config: ConsumerConfigInput,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let event_stream_name = stream_name.clone();
    with_stream(
        state,
        connection_id,
        &stream_name,
        evt_tx,
        BackendOperation::UpdateConsumer,
        |stream| async move {
            match stream.update_consumer(config.into_async_nats_pull()).await {
                Ok(consumer) => {
                    let _ = evt_tx
                        .send(BackendEvent::ConsumerUpdated {
                            connection_id,
                            stream: event_stream_name,
                            consumer: ConsumerInfo::from_info(consumer.cached_info()),
                        })
                        .await;
                }
                Err(e) => {
                    send_err(
                        evt_tx,
                        connection_id,
                        BackendOperation::UpdateConsumer,
                        e.to_string(),
                    )
                    .await
                }
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
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let operation = BackendOperation::FetchConsumerMessages;
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
                    send_err(evt_tx, connection_id, operation, e.to_string()).await;
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
                    )
                    .await;
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
                    send_err(evt_tx, connection_id, operation, e.to_string()).await;
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
                        send_err(evt_tx, connection_id, operation, e.to_string()).await;
                        return;
                    }
                };
                match async_nats::jetstream::message::StreamMessage::try_from(msg.message.clone()) {
                    Ok(stream_msg) => {
                        messages.push(StreamMessageInfo::from_stream_message(&stream_msg))
                    }
                    Err(e) => tracing::debug!(%e, "Failed to convert inspector message"),
                }
                if messages.len() >= batch {
                    break;
                }
            }
            drop(fetch_stream);
            cleanup_inspector_consumer(&stream, &inspector_name).await;

            let _ = evt_tx
                .send(BackendEvent::ConsumerMessagesFetched {
                    connection_id,
                    stream: stream_name,
                    consumer: consumer_name,
                    messages,
                })
                .await;
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
    evt_tx: &mpsc::Sender<BackendEvent>,
    operation: BackendOperation,
    f: F,
) where
    F: FnOnce(async_nats::jetstream::stream::Stream) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    if let Some(client) = state.clients.get(&connection_id) {
        let js = async_nats::jetstream::new(client.clone());
        match js.get_stream(stream_name).await {
            Ok(stream) => f(stream).await,
            Err(e) => send_err(evt_tx, connection_id, operation, e.to_string()).await,
        }
    } else {
        send_err(
            evt_tx,
            connection_id,
            operation,
            "Not connected".to_string(),
        )
        .await;
    }
}

async fn send_err(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
    message: String,
) {
    let _ = evt_tx
        .send(BackendEvent::Error {
            connection_id: Some(connection_id),
            backend_id: None,
            operation,
            message,
            context: None,
        })
        .await;
}
