use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation};
use crate::models::{StreamConfigInput, StreamInfo, StreamMessageInfo};

use super::state::WorkerState;

pub(crate) async fn handle_list_streams(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = jetstream(state, connection_id, evt_tx, BackendOperation::ListStreams).await
    else {
        return;
    };

    let mut stream_iter = js.streams();
    let mut list = Vec::new();
    while let Some(result) = stream_iter.next().await {
        match result {
            Ok(info) => list.push(StreamInfo::from_info(&info)),
            Err(e) => {
                tracing::warn!(%e, "Error iterating streams");
                break;
            }
        }
    }
    let _ = evt_tx
        .send(BackendEvent::StreamsListed {
            connection_id,
            streams: list,
        })
        .await;
}

pub(crate) async fn handle_create_stream(
    state: &WorkerState,
    connection_id: u64,
    config: StreamConfigInput,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = jetstream(state, connection_id, evt_tx, BackendOperation::CreateStream).await
    else {
        return;
    };

    match js.create_stream(config.into_async_nats()).await {
        Ok(stream) => {
            let _ = evt_tx
                .send(BackendEvent::StreamCreated {
                    connection_id,
                    stream: StreamInfo::from_info(stream.cached_info()),
                })
                .await;
        }
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::CreateStream,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) async fn handle_update_stream(
    state: &WorkerState,
    connection_id: u64,
    config: StreamConfigInput,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = jetstream(state, connection_id, evt_tx, BackendOperation::UpdateStream).await
    else {
        return;
    };

    match js.update_stream(config.into_async_nats()).await {
        Ok(info) => {
            let _ = evt_tx
                .send(BackendEvent::StreamUpdated {
                    connection_id,
                    stream: StreamInfo::from_info(&info),
                })
                .await;
        }
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::UpdateStream,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) async fn handle_delete_stream(
    state: &WorkerState,
    connection_id: u64,
    name: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = jetstream(state, connection_id, evt_tx, BackendOperation::DeleteStream).await
    else {
        return;
    };

    match js.delete_stream(&name).await {
        Ok(_) => {
            let _ = evt_tx
                .send(BackendEvent::StreamDeleted {
                    connection_id,
                    name,
                })
                .await;
        }
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::DeleteStream,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) async fn handle_purge_stream(
    state: &WorkerState,
    connection_id: u64,
    name: String,
    filter: Option<String>,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(stream) = open_stream(
        state,
        connection_id,
        &name,
        evt_tx,
        BackendOperation::PurgeStream,
    )
    .await
    else {
        return;
    };

    let result = if let Some(filter_subject) = filter {
        stream.purge().filter(filter_subject.as_str()).await
    } else {
        stream.purge().await
    };

    match result {
        Ok(resp) => {
            let _ = evt_tx
                .send(BackendEvent::StreamPurged {
                    connection_id,
                    name,
                    purged: resp.purged,
                })
                .await;
        }
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::PurgeStream,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) struct GetMessagesParams {
    pub stream_name: String,
    pub start_sequence: Option<u64>,
    pub subject_filter: Option<String>,
    pub start_time: Option<String>,
    pub batch_size: u64,
}

pub(crate) async fn handle_get_messages(
    state: &WorkerState,
    connection_id: u64,
    params: GetMessagesParams,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    tracing::info!(
        connection_id,
        stream = %params.stream_name,
        start_sequence = ?params.start_sequence,
        subject_filter = ?params.subject_filter,
        start_time = ?params.start_time,
        batch_size = params.batch_size,
        "Fetching stream messages"
    );
    let Some(stream) = open_stream(
        state,
        connection_id,
        &params.stream_name,
        evt_tx,
        BackendOperation::GetStreamMessages,
    )
    .await
    else {
        return;
    };

    // Time-based filtering via ephemeral pull consumer
    if let Some(time_str) = params.start_time {
        handle_get_messages_by_time(
            &stream,
            connection_id,
            &params.stream_name,
            &time_str,
            params.subject_filter,
            params.batch_size,
            evt_tx,
        )
        .await;
        return;
    }

    let info = stream.cached_info();
    let first = params.start_sequence.unwrap_or(info.state.first_sequence);
    let last = info.state.last_sequence;
    let mut messages = Vec::new();

    if let Some(filter) = params.subject_filter {
        let mut seq = first;
        while (messages.len() as u64) < params.batch_size {
            match stream.get_first_raw_message_by_subject(&filter, seq).await {
                Ok(raw) => {
                    seq = raw.sequence + 1;
                    messages.push(StreamMessageInfo::from_stream_message(&raw));
                }
                Err(_) => break,
            }
        }
    } else {
        let mut seq = first;
        while seq <= last && (messages.len() as u64) < params.batch_size {
            if let Ok(raw) = stream.get_raw_message(seq).await {
                messages.push(StreamMessageInfo::from_stream_message(&raw));
            }
            seq += 1;
        }
    }

    let message_count = messages.len();
    tracing::info!(connection_id, stream = %params.stream_name, message_count, "Fetched stream messages");
    let _ = evt_tx
        .send(BackendEvent::StreamMessagesFetched {
            connection_id,
            stream: params.stream_name,
            messages,
        })
        .await;
}

async fn handle_get_messages_by_time(
    stream: &async_nats::jetstream::stream::Stream,
    connection_id: u64,
    stream_name: &str,
    time_str: &str,
    subject_filter: Option<String>,
    batch_size: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    use async_nats::jetstream::consumer::{self, DeliverPolicy};
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    let start = match OffsetDateTime::parse(time_str, &Rfc3339) {
        Ok(t) => t,
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::GetStreamMessages,
                format!("Invalid time format (use RFC3339): {e}"),
            )
            .await;
            return;
        }
    };

    let mut config = consumer::pull::Config {
        deliver_policy: DeliverPolicy::ByStartTime { start_time: start },
        inactive_threshold: std::time::Duration::from_secs(10),
        ..Default::default()
    };
    if let Some(filter) = subject_filter {
        config.filter_subject = filter;
    }

    let consumer = match stream.create_consumer(config).await {
        Ok(c) => c,
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::GetStreamMessages,
                format!("Failed to create time-filter consumer: {e}"),
            )
            .await;
            return;
        }
    };

    let mut messages = Vec::new();
    let mut batch = match consumer
        .fetch()
        .max_messages(batch_size as usize)
        .expires(std::time::Duration::from_secs(5))
        .messages()
        .await
    {
        Ok(b) => b,
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::GetStreamMessages,
                format!("Fetch error: {e}"),
            )
            .await;
            return;
        }
    };

    while let Some(Ok(msg)) = batch.next().await {
        let info = msg.info().ok();
        let seq = info.as_ref().map(|i| i.stream_sequence).unwrap_or(0);
        let time_val = info
            .as_ref()
            .map(|i| {
                i.published
                    .format(&Rfc3339)
                    .unwrap_or_else(|_| i.published.to_string())
            })
            .unwrap_or_default();
        let headers = super::helpers::extract_headers(&msg.message.headers);
        messages.push(StreamMessageInfo {
            sequence: seq,
            subject: msg.message.subject.to_string(),
            payload: msg.message.payload.to_vec(),
            headers,
            time: time_val,
        });
    }

    // Ephemeral consumer auto-deletes on timeout; no explicit cleanup needed.
    let message_count = messages.len();
    tracing::info!(connection_id, stream = %stream_name, message_count, "Fetched stream messages by time");
    let _ = evt_tx
        .send(BackendEvent::StreamMessagesFetched {
            connection_id,
            stream: stream_name.to_string(),
            messages,
        })
        .await;
}

pub(crate) async fn handle_delete_message(
    state: &WorkerState,
    connection_id: u64,
    stream_name: String,
    sequence: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(stream) = open_stream(
        state,
        connection_id,
        &stream_name,
        evt_tx,
        BackendOperation::DeleteStreamMessage,
    )
    .await
    else {
        return;
    };

    match stream.delete_message(sequence).await {
        Ok(_) => {
            let _ = evt_tx
                .send(BackendEvent::StreamMessageDeleted {
                    connection_id,
                    stream: stream_name,
                    sequence,
                })
                .await;
        }
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::DeleteStreamMessage,
                e.to_string(),
            )
            .await
        }
    }
}

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

async fn open_stream(
    state: &WorkerState,
    connection_id: u64,
    stream_name: &str,
    evt_tx: &mpsc::Sender<BackendEvent>,
    operation: BackendOperation,
) -> Option<async_nats::jetstream::stream::Stream> {
    let js = jetstream(state, connection_id, evt_tx, operation).await?;
    match js.get_stream(stream_name).await {
        Ok(stream) => Some(stream),
        Err(e) => {
            send_err(evt_tx, connection_id, operation, e.to_string()).await;
            None
        }
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
