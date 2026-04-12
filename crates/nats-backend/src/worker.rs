use std::collections::HashMap;

use async_nats::Client;
use tokio::sync::mpsc;

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
            BackendCommand::Publish { connection_id, .. } => {
                // TODO: Implement publish
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "publish".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::Subscribe { connection_id, .. } => {
                // TODO: Implement subscribe
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "subscribe".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::Unsubscribe { connection_id, .. } => {
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "unsubscribe".to_string(),
                    message: "Not yet implemented".to_string(),
                });
            }
            BackendCommand::Request { connection_id, .. } => {
                // TODO: Implement request-reply
                let _ = evt_tx.send(BackendEvent::Error {
                    connection_id: Some(connection_id),
                    operation: "request".to_string(),
                    message: "Not yet implemented".to_string(),
                });
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
