use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation};
use crate::models::{BackendErrorContext, ConsumerConfigInput, ConsumerInfo, StreamMessageInfo};

use super::state::WorkerState;

static INSPECTOR_COUNTER: AtomicU64 = AtomicU64::new(0);
const WORKQUEUE_INSPECTOR_UNSUPPORTED_REASON: &str = "workqueue_inspector_not_supported";
const WORKQUEUE_INSPECTOR_UNSUPPORTED_MESSAGE: &str = "Safe WorkQueue preview is unavailable because NATS does not allow a second overlapping filtered consumer on a WorkQueue stream";

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
            let consumer_config = match config.into_async_nats_pull() {
                Ok(config) => config,
                Err(e) => {
                    send_err(evt_tx, connection_id, BackendOperation::CreateConsumer, e).await;
                    return;
                }
            };
            match stream.create_consumer(consumer_config).await {
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
            let consumer_config = match config.into_async_nats_pull() {
                Ok(config) => config,
                Err(e) => {
                    send_err(evt_tx, connection_id, BackendOperation::UpdateConsumer, e).await;
                    return;
                }
            };
            match stream.update_consumer(consumer_config).await {
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
        |mut stream| async move {
            let is_workqueue = match stream.info().await {
                Ok(info) => is_workqueue_retention(info.config.retention),
                Err(e) => {
                    send_err(evt_tx, connection_id, operation, e.to_string()).await;
                    return;
                }
            };
            if is_workqueue {
                send_err_with_context(
                    evt_tx,
                    connection_id,
                    operation,
                    WORKQUEUE_INSPECTOR_UNSUPPORTED_MESSAGE.to_string(),
                    Some(workqueue_preview_limitation_context(
                        &stream_name,
                        &consumer_name,
                    )),
                )
                .await;
                return;
            }

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
            let inspector_config = inspector_consumer_config(filter, filters, unique_id);

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

fn is_workqueue_retention(policy: async_nats::jetstream::stream::RetentionPolicy) -> bool {
    matches!(
        policy,
        async_nats::jetstream::stream::RetentionPolicy::WorkQueue
    )
}

fn workqueue_preview_limitation_context(
    stream_name: &str,
    consumer_name: &str,
) -> BackendErrorContext {
    BackendErrorContext::WorkQueueConsumerPreview {
        stream: stream_name.to_string(),
        consumer: consumer_name.to_string(),
        reason: WORKQUEUE_INSPECTOR_UNSUPPORTED_REASON.to_string(),
    }
}

fn inspector_consumer_config(
    filter_subject: String,
    filter_subjects: Vec<String>,
    unique_id: u64,
) -> async_nats::jetstream::consumer::pull::Config {
    async_nats::jetstream::consumer::pull::Config {
        name: Some(format!("_easynats_inspect_{unique_id}")),
        filter_subject,
        filter_subjects,
        deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
        memory_storage: true,
        inactive_threshold: std::time::Duration::from_secs(10),
        ..Default::default()
    }
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
    send_err_with_context(evt_tx, connection_id, operation, message, None).await;
}

async fn send_err_with_context(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
    message: String,
    context: Option<BackendErrorContext>,
) {
    let _ = evt_tx
        .send(BackendEvent::Error {
            connection_id: Some(connection_id),
            backend_id: None,
            operation,
            message,
            context,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use async_nats::jetstream::stream::RetentionPolicy;

    use super::*;

    #[test]
    fn workqueue_retention_is_detected_before_inspector_creation() {
        assert!(is_workqueue_retention(RetentionPolicy::WorkQueue));
        assert!(!is_workqueue_retention(RetentionPolicy::Limits));
        assert!(!is_workqueue_retention(RetentionPolicy::Interest));
    }

    #[test]
    fn workqueue_preview_limitation_context_has_stable_reason() {
        let context = workqueue_preview_limitation_context("ORDERS", "worker-a");

        assert_eq!(
            context,
            BackendErrorContext::WorkQueueConsumerPreview {
                stream: "ORDERS".to_string(),
                consumer: "worker-a".to_string(),
                reason: WORKQUEUE_INSPECTOR_UNSUPPORTED_REASON.to_string(),
            }
        );
    }

    #[test]
    fn inspector_config_mirrors_consumer_filter_and_uses_traceable_name() {
        let config = inspector_consumer_config(
            "orders.*".to_string(),
            vec!["orders.created".to_string(), "orders.updated".to_string()],
            42,
        );

        assert_eq!(config.name.as_deref(), Some("_easynats_inspect_42"));
        assert_eq!(config.filter_subject, "orders.*");
        assert_eq!(
            config.filter_subjects,
            vec!["orders.created".to_string(), "orders.updated".to_string()]
        );
        assert!(config.memory_storage);
        assert_eq!(
            config.inactive_threshold,
            std::time::Duration::from_secs(10)
        );
    }
}
