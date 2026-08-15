use async_nats::Client;
use tokio::sync::mpsc;

use crate::connection::{AuthMethod, ConnectionConfig};
use crate::event::{BackendEvent, ConnectionStatusKind};

use super::state::WorkerState;

pub(crate) async fn handle_connect(
    state: &mut WorkerState,
    config: ConnectionConfig,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let id = config.id;
    let _ = evt_tx
        .send(BackendEvent::ConnectionStatus {
            connection_id: id,
            status: ConnectionStatusKind::Connecting,
        })
        .await;

    match do_connect(&config, Some(evt_tx)).await {
        Ok(client) => {
            state.clients.insert(id, client);
            let _ = evt_tx
                .send(BackendEvent::ConnectionStatus {
                    connection_id: id,
                    status: ConnectionStatusKind::Connected,
                })
                .await;
        }
        Err(e) => {
            let _ = evt_tx
                .send(BackendEvent::ConnectionStatus {
                    connection_id: id,
                    status: ConnectionStatusKind::Error(e.to_string()),
                })
                .await;
        }
    }
}

pub(crate) async fn handle_disconnect(
    state: &mut WorkerState,
    id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    state.subscriptions.retain(|(cid, _, _), handle| {
        if *cid == id {
            handle.abort();
            false
        } else {
            true
        }
    });

    state.kv_tasks.retain(|(cid, _), (_, handle)| {
        if *cid == id {
            handle.abort();
            false
        } else {
            true
        }
    });

    if let Some(client) = state.clients.remove(&id)
        && let Err(e) = client.drain().await
    {
        tracing::warn!(id, %e, "Error draining connection");
    }

    let _ = evt_tx
        .send(BackendEvent::ConnectionStatus {
            connection_id: id,
            status: ConnectionStatusKind::Disconnected,
        })
        .await;
}

pub(crate) async fn handle_test_connection(
    config: ConnectionConfig,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let connection_id = config.id;
    let result = match do_connect(&config, None).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    };
    let _ = evt_tx
        .send(BackendEvent::ConnectionTestResult {
            connection_id,
            result,
        })
        .await;
}

async fn do_connect(
    config: &ConnectionConfig,
    evt_tx: Option<&mpsc::Sender<BackendEvent>>,
) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
    let id = config.id;
    let event_tx = evt_tx.cloned();

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

    if config.tls_first {
        opts = opts.require_tls(true).tls_first();
    } else if config.tls_enabled {
        opts = opts.require_tls(true);
    }

    opts = opts.name(&config.name).event_callback(move |event| {
        let tx = event_tx.clone();
        async move {
            let Some(tx) = tx else {
                return;
            };
            let status = match event {
                async_nats::Event::Connected => ConnectionStatusKind::Connected,
                async_nats::Event::Disconnected => ConnectionStatusKind::Disconnected,
                async_nats::Event::ClientError(e) => ConnectionStatusKind::Error(e.to_string()),
                other => {
                    tracing::debug!(connection_id = id, %other, "NATS event");
                    return;
                }
            };
            let _ = tx
                .send(BackendEvent::ConnectionStatus {
                    connection_id: id,
                    status,
                })
                .await;
        }
    });

    let addrs: Vec<async_nats::ServerAddr> = config
        .urls
        .iter()
        .map(|u| u.parse())
        .collect::<Result<_, _>>()?;

    Ok(opts.connect(&addrs[..]).await?)
}
