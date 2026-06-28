use std::collections::hash_map::Entry;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation, MessageData, RequestFailureKind};

use super::helpers::{build_header_map, extract_headers};
use super::state::WorkerState;

const MAX_BATCH_SIZE: usize = 256;

pub(crate) struct RequestParams {
    pub connection_id: u64,
    pub backend_id: u64,
    pub request_id: u64,
    pub subject: String,
    pub payload: Vec<u8>,
    pub headers: Option<Vec<(String, String)>>,
    pub timeout_ms: u64,
}

pub(crate) struct ReplyParams {
    pub connection_id: u64,
    pub backend_id: u64,
    pub reply_id: u64,
    pub reply_to: String,
    pub payload: Vec<u8>,
    pub headers: Option<Vec<(String, String)>>,
}

pub(crate) async fn handle_publish(
    state: &WorkerState,
    connection_id: u64,
    subject: String,
    payload: Vec<u8>,
    headers: Option<Vec<(String, String)>>,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    if let Some(client) = state.clients.get(&connection_id) {
        let payload = Bytes::from(payload);
        let headers = match build_optional_header_map(headers) {
            Ok(headers) => headers,
            Err(message) => {
                send_err(
                    evt_tx,
                    connection_id,
                    None,
                    BackendOperation::Publish,
                    message,
                )
                .await;
                return;
            }
        };
        let result = if let Some(hdrs) = headers {
            client.publish_with_headers(subject, hdrs, payload).await
        } else {
            client.publish(subject, payload).await
        };
        match result {
            Ok(()) => send_ok(evt_tx, connection_id, BackendOperation::Publish).await,
            Err(e) => {
                send_err(
                    evt_tx,
                    connection_id,
                    None,
                    BackendOperation::Publish,
                    e.to_string(),
                )
                .await
            }
        }
    } else {
        send_not_connected(evt_tx, connection_id, None, BackendOperation::Publish).await;
    }
}

pub(crate) async fn handle_subscribe(
    state: &mut WorkerState,
    connection_id: u64,
    backend_id: u64,
    subject: String,
    cancel: crate::TaskCancellation,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let key = (connection_id, backend_id, subject.clone());
    match state.subscriptions.entry(key) {
        Entry::Occupied(_) => {
            send_err(
                evt_tx,
                connection_id,
                Some(backend_id),
                BackendOperation::Subscribe,
                format!("Already subscribed to {subject}"),
            )
            .await
        }
        Entry::Vacant(vacant) => {
            if let Some(client) = state.clients.get(&connection_id) {
                match client.subscribe(subject.clone()).await {
                    Ok(mut subscriber) => {
                        let tx = evt_tx.clone();
                        let token = cancel.into_token();
                        let handle = tokio::spawn(async move {
                            loop {
                                tokio::select! {
                                    msg = subscriber.next() => {
                                        match msg {
                                            Some(msg) => {
                                                let mut messages = vec![to_message_data(msg)];
                                                let deadline = tokio::time::Instant::now() + Duration::from_millis(5);
                                                let mut stream_ended = false;
                                                loop {
                                                    tokio::select! {
                                                        _ = token.cancelled() => return,
                                                        next = tokio::time::timeout_at(deadline, subscriber.next()) => {
                                                            match next {
                                                                Ok(Some(msg)) => {
                                                    messages.push(to_message_data(msg));
                                                    if messages.len() >= MAX_BATCH_SIZE {
                                                        break;
                                                    }
                                                }
                                                                Ok(None) => {
                                                                    stream_ended = true;
                                                                    break;
                                                                }
                                                                Err(_) => break,
                                                            }
                                                        }
                                                    }
                                                }
                                                if tx.send(BackendEvent::MessageBatch {
                                                    connection_id,
                                                    backend_id,
                                                    messages,
                                                }).await.is_err() {
                                                    break;
                                                }
                                                if stream_ended {
                                                    break;
                                                }
                                            }
                                            None => break, // stream ended
                                        }
                                    }
                                    _ = token.cancelled() => break,
                                }
                            }
                        });
                        vacant.insert(handle);
                        send_ok(evt_tx, connection_id, BackendOperation::Subscribe).await;
                    }
                    Err(e) => {
                        send_err(
                            evt_tx,
                            connection_id,
                            Some(backend_id),
                            BackendOperation::Subscribe,
                            e.to_string(),
                        )
                        .await
                    }
                }
            } else {
                send_not_connected(
                    evt_tx,
                    connection_id,
                    Some(backend_id),
                    BackendOperation::Subscribe,
                )
                .await;
            }
        }
    }
}

pub(crate) async fn handle_unsubscribe(
    state: &mut WorkerState,
    connection_id: u64,
    backend_id: u64,
    subject: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    if let Some(handle) = state
        .subscriptions
        .remove(&(connection_id, backend_id, subject.clone()))
    {
        handle.abort();
        send_ok(evt_tx, connection_id, BackendOperation::Unsubscribe).await;
    } else {
        send_err(
            evt_tx,
            connection_id,
            Some(backend_id),
            BackendOperation::Unsubscribe,
            format!("Not subscribed to {subject}"),
        )
        .await;
    }
}

pub(crate) async fn handle_request(
    state: &WorkerState,
    params: RequestParams,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let RequestParams {
        connection_id,
        backend_id,
        request_id,
        subject,
        payload,
        headers,
        timeout_ms,
    } = params;
    if let Some(client) = state.clients.get(&connection_id) {
        let headers = match build_optional_header_map(headers) {
            Ok(headers) => headers,
            Err(message) => {
                send_request_failed(
                    evt_tx,
                    connection_id,
                    backend_id,
                    request_id,
                    message,
                    RequestFailureKind::Other,
                )
                .await;
                return;
            }
        };
        let client = client.clone();
        let evt_tx = evt_tx.clone();
        let request_subject = subject.clone();
        tokio::spawn(async move {
            let payload = Bytes::from(payload);
            let timeout = std::time::Duration::from_millis(timeout_ms);
            let result = tokio::time::timeout(timeout, async {
                if let Some(hdrs) = headers {
                    client.request_with_headers(subject, hdrs, payload).await
                } else {
                    client.request(subject, payload).await
                }
            })
            .await;
            match result {
                Ok(Ok(msg)) => {
                    let _ = evt_tx
                        .send(BackendEvent::RequestResponse {
                            connection_id,
                            backend_id,
                            request_id,
                            subject: Some(request_subject),
                            payload: msg.payload.to_vec(),
                            headers: extract_headers(&msg.headers),
                        })
                        .await;
                }
                Ok(Err(e)) => {
                    let kind = request_failure_kind(e.kind());
                    send_request_failed(
                        &evt_tx,
                        connection_id,
                        backend_id,
                        request_id,
                        e.to_string(),
                        kind,
                    )
                    .await;
                }
                Err(_) => {
                    send_request_failed(
                        &evt_tx,
                        connection_id,
                        backend_id,
                        request_id,
                        "Request timed out".to_string(),
                        RequestFailureKind::TimedOut,
                    )
                    .await;
                }
            }
        });
    } else {
        send_request_failed(
            evt_tx,
            connection_id,
            backend_id,
            request_id,
            "Not connected".to_string(),
            RequestFailureKind::Other,
        )
        .await;
    }
}

pub(crate) async fn handle_reply(
    state: &WorkerState,
    params: ReplyParams,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let ReplyParams {
        connection_id,
        backend_id,
        reply_id,
        reply_to,
        payload,
        headers,
    } = params;

    if let Some(client) = state.clients.get(&connection_id) {
        let headers = match build_optional_header_map(headers) {
            Ok(headers) => headers,
            Err(message) => {
                send_reply_failed(evt_tx, connection_id, backend_id, reply_id, message).await;
                return;
            }
        };
        let client = client.clone();
        let evt_tx = evt_tx.clone();
        tokio::spawn(async move {
            let payload = Bytes::from(payload);
            let result = if let Some(hdrs) = headers {
                client
                    .publish_with_headers(reply_to.clone(), hdrs, payload)
                    .await
            } else {
                client.publish(reply_to.clone(), payload).await
            };

            match result {
                Ok(()) => {
                    let _ = evt_tx
                        .send(BackendEvent::Replied {
                            connection_id,
                            backend_id,
                            reply_id,
                            subject: reply_to,
                        })
                        .await;
                }
                Err(e) => {
                    send_reply_failed(&evt_tx, connection_id, backend_id, reply_id, e.to_string())
                        .await;
                }
            }
        });
    } else {
        send_reply_failed(
            evt_tx,
            connection_id,
            backend_id,
            reply_id,
            "Not connected".to_string(),
        )
        .await;
    }
}

fn to_message_data(msg: async_nats::Message) -> MessageData {
    MessageData {
        subject: msg.subject.to_string(),
        reply: msg.reply.map(|reply| reply.to_string()),
        headers: extract_headers(&msg.headers),
        payload: msg.payload.to_vec(),
        timestamp: SystemTime::now(),
    }
}

fn request_failure_kind(kind: async_nats::RequestErrorKind) -> RequestFailureKind {
    match kind {
        async_nats::RequestErrorKind::TimedOut => RequestFailureKind::TimedOut,
        async_nats::RequestErrorKind::NoResponders => RequestFailureKind::NoResponders,
        async_nats::RequestErrorKind::InvalidSubject | async_nats::RequestErrorKind::Other => {
            RequestFailureKind::Other
        }
    }
}

fn build_optional_header_map(
    headers: Option<Vec<(String, String)>>,
) -> Result<Option<async_nats::HeaderMap>, String> {
    headers.as_deref().map(build_header_map).transpose()
}

async fn send_ok(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
) {
    let _ = evt_tx
        .send(BackendEvent::OperationSucceeded {
            connection_id,
            operation,
        })
        .await;
}

async fn send_err(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    backend_id: Option<u64>,
    operation: BackendOperation,
    message: String,
) {
    let _ = evt_tx
        .send(BackendEvent::Error {
            connection_id: Some(connection_id),
            backend_id,
            operation,
            message,
            context: None,
        })
        .await;
}

async fn send_not_connected(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    backend_id: Option<u64>,
    operation: BackendOperation,
) {
    send_err(
        evt_tx,
        connection_id,
        backend_id,
        operation,
        "Not connected".to_string(),
    )
    .await;
}

async fn send_request_failed(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    backend_id: u64,
    request_id: u64,
    message: String,
    kind: RequestFailureKind,
) {
    let _ = evt_tx
        .send(BackendEvent::RequestFailed {
            connection_id,
            backend_id,
            request_id,
            message,
            kind,
        })
        .await;
}

async fn send_reply_failed(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    backend_id: u64,
    reply_id: u64,
    message: String,
) {
    let _ = evt_tx
        .send(BackendEvent::ReplyFailed {
            connection_id,
            backend_id,
            reply_id,
            message,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_failure_kind_preserves_user_visible_categories() {
        assert_eq!(
            request_failure_kind(async_nats::RequestErrorKind::TimedOut),
            RequestFailureKind::TimedOut
        );
        assert_eq!(
            request_failure_kind(async_nats::RequestErrorKind::NoResponders),
            RequestFailureKind::NoResponders
        );
        assert_eq!(
            request_failure_kind(async_nats::RequestErrorKind::InvalidSubject),
            RequestFailureKind::Other
        );
        assert_eq!(
            request_failure_kind(async_nats::RequestErrorKind::Other),
            RequestFailureKind::Other
        );
    }
}
