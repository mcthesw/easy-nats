use std::collections::hash_map::Entry;
use std::time::SystemTime;

use bytes::Bytes;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::helpers::{build_header_map, extract_headers};
use super::state::WorkerState;

pub(crate) async fn handle_publish(
    state: &WorkerState,
    connection_id: u64,
    subject: String,
    payload: Vec<u8>,
    headers: Option<Vec<(String, String)>>,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    if let Some(client) = state.clients.get(&connection_id) {
        let payload = Bytes::from(payload);
        let result = if let Some(hdrs) = headers {
            client
                .publish_with_headers(subject, build_header_map(&hdrs), payload)
                .await
        } else {
            client.publish(subject, payload).await
        };
        match result {
            Ok(()) => send_ok(evt_tx, connection_id, "publish", serde_json::Value::Null),
            Err(e) => send_err(evt_tx, connection_id, "publish", e.to_string()),
        }
    } else {
        send_not_connected(evt_tx, connection_id, "publish");
    }
}

pub(crate) async fn handle_subscribe(
    state: &mut WorkerState,
    connection_id: u64,
    backend_id: u64,
    subject: String,
    cancel: crate::TaskCancellation,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let key = (connection_id, backend_id, subject.clone());
    match state.subscriptions.entry(key) {
        Entry::Occupied(_) => send_err(
            evt_tx,
            connection_id,
            "subscribe",
            format!("Already subscribed to {subject}"),
        ),
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
                                                let _ = tx.send(BackendEvent::MessageReceived {
                                                    connection_id,
                                                    backend_id,
                                                    subject: msg.subject.to_string(),
                                                    reply: msg.reply.map(|r| r.to_string()),
                                                    headers: extract_headers(&msg.headers),
                                                    payload: msg.payload.to_vec(),
                                                    timestamp: SystemTime::now(),
                                                });
                                            }
                                            None => break, // stream ended
                                        }
                                    }
                                    _ = token.cancelled() => break,
                                }
                            }
                        });
                        vacant.insert(handle);
                        send_ok(
                            evt_tx,
                            connection_id,
                            "subscribe",
                            serde_json::json!({ "subject": subject }),
                        );
                    }
                    Err(e) => send_err(evt_tx, connection_id, "subscribe", e.to_string()),
                }
            } else {
                send_not_connected(evt_tx, connection_id, "subscribe");
            }
        }
    }
}

pub(crate) async fn handle_unsubscribe(
    state: &mut WorkerState,
    connection_id: u64,
    backend_id: u64,
    subject: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    if let Some(handle) = state
        .subscriptions
        .remove(&(connection_id, backend_id, subject.clone()))
    {
        handle.abort();
        send_ok(
            evt_tx,
            connection_id,
            "unsubscribe",
            serde_json::json!({ "subject": subject }),
        );
    } else {
        send_err(
            evt_tx,
            connection_id,
            "unsubscribe",
            format!("Not subscribed to {subject}"),
        );
    }
}

pub(crate) async fn handle_request(
    state: &WorkerState,
    connection_id: u64,
    subject: String,
    payload: Vec<u8>,
    headers: Option<Vec<(String, String)>>,
    timeout_ms: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    if let Some(client) = state.clients.get(&connection_id) {
        let payload = Bytes::from(payload);
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let result = tokio::time::timeout(timeout, async {
            if let Some(hdrs) = headers {
                client
                    .request_with_headers(subject, build_header_map(&hdrs), payload)
                    .await
            } else {
                client.request(subject, payload).await
            }
        })
        .await;
        match result {
            Ok(Ok(msg)) => {
                let _ = evt_tx.send(BackendEvent::RequestResponse {
                    connection_id,
                    payload: msg.payload.to_vec(),
                    headers: extract_headers(&msg.headers),
                });
            }
            Ok(Err(e)) => send_err(evt_tx, connection_id, "request", e.to_string()),
            Err(_) => send_err(
                evt_tx,
                connection_id,
                "request",
                "Request timed out".to_string(),
            ),
        }
    } else {
        send_not_connected(evt_tx, connection_id, "request");
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
