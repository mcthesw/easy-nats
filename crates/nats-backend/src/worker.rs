use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::time::SystemTime;

use async_nats::Client;
use bytes::Bytes;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::command::BackendCommand;
use crate::connection::{AuthMethod, ConnectionConfig};
use crate::event::{BackendEvent, ConnectionStatusKind};

/// Main async worker loop that receives commands and dispatches to async-nats.
pub async fn run_worker(
    mut cmd_rx: mpsc::UnboundedReceiver<BackendCommand>,
    evt_tx: mpsc::UnboundedSender<BackendEvent>,
) {
    tracing::info!("Backend worker started");

    let mut clients: HashMap<u64, Client> = HashMap::new();
    // Key: (connection_id, subject) → subscription task handle
    let mut subscriptions: HashMap<(u64, String), JoinHandle<()>> = HashMap::new();

    while let Some(cmd) = cmd_rx.recv().await {
        tracing::debug!(?cmd, "Received command");

        match cmd {
            BackendCommand::Connect { config } => {
                let id = config.id;
                let _ = evt_tx.send(BackendEvent::ConnectionStatus {
                    connection_id: id,
                    status: ConnectionStatusKind::Connecting,
                });

                match do_connect(&config, &evt_tx).await {
                    Ok(client) => {
                        clients.insert(id, client);
                        let _ = evt_tx.send(BackendEvent::ConnectionStatus {
                            connection_id: id,
                            status: ConnectionStatusKind::Connected,
                        });
                    }
                    Err(e) => {
                        let _ = evt_tx.send(BackendEvent::ConnectionStatus {
                            connection_id: id,
                            status: ConnectionStatusKind::Error(e.to_string()),
                        });
                    }
                }
            }
            BackendCommand::Disconnect { id } => {
                // Abort all subscriptions for this connection
                subscriptions.retain(|(cid, _), handle| {
                    if *cid == id {
                        handle.abort();
                        false
                    } else {
                        true
                    }
                });
                if let Some(client) = clients.remove(&id)
                    && let Err(e) = client.drain().await
                {
                    tracing::warn!(id, %e, "Error draining connection");
                }
                let _ = evt_tx.send(BackendEvent::ConnectionStatus {
                    connection_id: id,
                    status: ConnectionStatusKind::Disconnected,
                });
            }
            BackendCommand::Publish {
                connection_id,
                subject,
                payload,
                headers,
            } => {
                if let Some(client) = clients.get(&connection_id) {
                    let payload = Bytes::from(payload);
                    let result = if let Some(hdrs) = headers {
                        client
                            .publish_with_headers(subject, build_header_map(&hdrs), payload)
                            .await
                    } else {
                        client.publish(subject, payload).await
                    };
                    match result {
                        Ok(()) => {
                            let _ = evt_tx.send(BackendEvent::OperationResult {
                                connection_id,
                                operation: "publish".to_string(),
                                data: serde_json::Value::Null,
                            });
                        }
                        Err(e) => {
                            let _ = evt_tx.send(BackendEvent::Error {
                                connection_id: Some(connection_id),
                                operation: "publish".to_string(),
                                message: e.to_string(),
                            });
                        }
                    }
                } else {
                    let _ = evt_tx.send(BackendEvent::Error {
                        connection_id: Some(connection_id),
                        operation: "publish".to_string(),
                        message: "Not connected".to_string(),
                    });
                }
            }
            BackendCommand::Subscribe {
                connection_id,
                subject,
            } => {
                let key = (connection_id, subject.clone());
                match subscriptions.entry(key) {
                    Entry::Occupied(_) => {
                        let _ = evt_tx.send(BackendEvent::Error {
                            connection_id: Some(connection_id),
                            operation: "subscribe".to_string(),
                            message: format!("Already subscribed to {subject}"),
                        });
                    }
                    Entry::Vacant(vacant) => {
                        if let Some(client) = clients.get(&connection_id) {
                            match client.subscribe(subject.clone()).await {
                                Ok(mut subscriber) => {
                                    let tx = evt_tx.clone();
                                    let subj = subject.clone();
                                    let handle = tokio::spawn(async move {
                                        while let Some(msg) = subscriber.next().await {
                                            let _ = tx.send(BackendEvent::MessageReceived {
                                                connection_id,
                                                subject: msg.subject.to_string(),
                                                reply: msg.reply.map(|r| r.to_string()),
                                                headers: extract_headers(&msg.headers),
                                                payload: msg.payload.to_vec(),
                                                timestamp: SystemTime::now(),
                                            });
                                        }
                                        tracing::debug!(
                                            connection_id,
                                            subject = %subj,
                                            "Subscription stream ended"
                                        );
                                    });
                                    vacant.insert(handle);
                                    let _ = evt_tx.send(BackendEvent::OperationResult {
                                        connection_id,
                                        operation: "subscribe".to_string(),
                                        data: serde_json::json!({ "subject": subject }),
                                    });
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(BackendEvent::Error {
                                        connection_id: Some(connection_id),
                                        operation: "subscribe".to_string(),
                                        message: e.to_string(),
                                    });
                                }
                            }
                        } else {
                            let _ = evt_tx.send(BackendEvent::Error {
                                connection_id: Some(connection_id),
                                operation: "subscribe".to_string(),
                                message: "Not connected".to_string(),
                            });
                        }
                    }
                }
            }
            BackendCommand::Unsubscribe {
                connection_id,
                subject,
            } => {
                let key = (connection_id, subject.clone());
                if let Some(handle) = subscriptions.remove(&key) {
                    handle.abort();
                    let _ = evt_tx.send(BackendEvent::OperationResult {
                        connection_id,
                        operation: "unsubscribe".to_string(),
                        data: serde_json::json!({ "subject": subject }),
                    });
                } else {
                    let _ = evt_tx.send(BackendEvent::Error {
                        connection_id: Some(connection_id),
                        operation: "unsubscribe".to_string(),
                        message: format!("Not subscribed to {subject}"),
                    });
                }
            }
            BackendCommand::Request {
                connection_id,
                subject,
                payload,
                headers,
                timeout_ms,
            } => {
                if let Some(client) = clients.get(&connection_id) {
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
                        Ok(Err(e)) => {
                            let _ = evt_tx.send(BackendEvent::Error {
                                connection_id: Some(connection_id),
                                operation: "request".to_string(),
                                message: e.to_string(),
                            });
                        }
                        Err(_) => {
                            let _ = evt_tx.send(BackendEvent::Error {
                                connection_id: Some(connection_id),
                                operation: "request".to_string(),
                                message: "Request timed out".to_string(),
                            });
                        }
                    }
                } else {
                    let _ = evt_tx.send(BackendEvent::Error {
                        connection_id: Some(connection_id),
                        operation: "request".to_string(),
                        message: "Not connected".to_string(),
                    });
                }
            }
            // JetStream commands
            BackendCommand::ListStreams { connection_id }
            | BackendCommand::ListKvBuckets { connection_id }
            | BackendCommand::ListObjectStoreBuckets { connection_id } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "list".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::CreateStream { connection_id, .. }
            | BackendCommand::UpdateStream { connection_id, .. }
            | BackendCommand::CreateKvBucket { connection_id, .. }
            | BackendCommand::CreateObjectStoreBucket { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "create".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::DeleteStream { connection_id, .. }
            | BackendCommand::DeleteKvBucket { connection_id, .. }
            | BackendCommand::DeleteObjectStoreBucket { connection_id, .. }
            | BackendCommand::DeleteConsumer { connection_id, .. }
            | BackendCommand::DeleteObject { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "delete".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::PurgeStream { connection_id, .. }
            | BackendCommand::PurgeKvEntry { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "purge".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::GetStreamMessages { connection_id, .. }
            | BackendCommand::ListConsumers { connection_id, .. }
            | BackendCommand::CreateConsumer { connection_id, .. }
            | BackendCommand::ListKvKeys { connection_id, .. }
            | BackendCommand::GetKvEntry { connection_id, .. }
            | BackendCommand::GetKvHistory { connection_id, .. }
            | BackendCommand::ListObjects { connection_id, .. }
            | BackendCommand::DownloadObject { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "query".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::DeleteStreamMessage { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "delete_message".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::PutKvEntry { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "put".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::DeleteKvEntry { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "delete_kv_entry".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::UploadObject { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "upload".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
        }
    }

    tracing::info!("Backend worker stopped");
}

/// Convert a list of key-value pairs into an async-nats HeaderMap.
pub fn build_header_map(headers: &[(String, String)]) -> async_nats::HeaderMap {
    let mut map = async_nats::HeaderMap::new();
    for (k, v) in headers {
        map.insert(k.as_str(), v.as_str());
    }
    map
}

/// Extract headers from an async-nats HeaderMap into a list of key-value pairs.
pub fn extract_headers(headers: &Option<async_nats::HeaderMap>) -> Vec<(String, String)> {
    let Some(map) = headers else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for (name, values) in map.iter() {
        for value in values.iter() {
            result.push((name.to_string(), value.to_string()));
        }
    }
    result
}

/// Build ConnectOptions from a ConnectionConfig and connect.
async fn do_connect(
    config: &ConnectionConfig,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
    let id = config.id;
    let event_tx = evt_tx.clone();

    let mut opts = match &config.auth {
        AuthMethod::None => async_nats::ConnectOptions::new(),
        AuthMethod::Token { token } => async_nats::ConnectOptions::with_token(token.clone()),
        AuthMethod::UserPassword { username, password } => {
            async_nats::ConnectOptions::with_user_and_password(username.clone(), password.clone())
        }
        AuthMethod::NKey { seed } => async_nats::ConnectOptions::with_nkey(seed.clone()),
        AuthMethod::CredentialsFile { path } => {
            async_nats::ConnectOptions::with_credentials_file(path).await?
        }
        AuthMethod::TlsClientCert {
            cert_path,
            key_path,
        } => async_nats::ConnectOptions::new()
            .add_client_certificate(cert_path.into(), key_path.into()),
    };

    if config.tls_enabled {
        opts = opts.require_tls(true);
    }

    opts = opts.name(&config.name).event_callback(move |event| {
        let tx = event_tx.clone();
        async move {
            let status = match event {
                async_nats::Event::Connected => ConnectionStatusKind::Connected,
                async_nats::Event::Disconnected => ConnectionStatusKind::Disconnected,
                async_nats::Event::ClientError(e) => ConnectionStatusKind::Error(e.to_string()),
                other => {
                    tracing::debug!(connection_id = id, %other, "NATS event");
                    return;
                }
            };
            let _ = tx.send(BackendEvent::ConnectionStatus {
                connection_id: id,
                status,
            });
        }
    });

    let addrs: Vec<async_nats::ServerAddr> = config
        .urls
        .iter()
        .map(|u| u.parse())
        .collect::<Result<_, _>>()?;

    let client = opts.connect(&addrs[..]).await?;
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_header_map_empty() {
        let headers: Vec<(String, String)> = vec![];
        let map = build_header_map(&headers);
        assert_eq!(map.iter().count(), 0);
    }

    #[test]
    fn test_build_header_map_single() {
        let headers = vec![("X-Key".to_string(), "Value1".to_string())];
        let map = build_header_map(&headers);
        assert_eq!(map.get("X-Key").unwrap().to_string(), "Value1");
    }

    #[test]
    fn test_build_header_map_multiple() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Request-Id".to_string(), "abc-123".to_string()),
        ];
        let map = build_header_map(&headers);
        assert_eq!(
            map.get("Content-Type").unwrap().to_string(),
            "application/json"
        );
        assert_eq!(map.get("X-Request-Id").unwrap().to_string(), "abc-123");
    }

    #[test]
    fn test_extract_headers_none() {
        let result = extract_headers(&None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_headers_roundtrip() {
        let original = vec![
            ("X-Foo".to_string(), "bar".to_string()),
            ("X-Baz".to_string(), "qux".to_string()),
        ];
        let map = build_header_map(&original);
        let extracted = extract_headers(&Some(map));
        assert_eq!(extracted.len(), 2);
        // Headers may be in different order, so check presence
        assert!(extracted.iter().any(|(k, v)| k == "X-Foo" && v == "bar"));
        assert!(extracted.iter().any(|(k, v)| k == "X-Baz" && v == "qux"));
    }
}
