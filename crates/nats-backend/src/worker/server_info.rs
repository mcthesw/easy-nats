use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation};
use crate::models::{JetStreamAccountInfoSnapshot, ServerInfoSnapshot};

use super::state::WorkerState;

pub(crate) async fn handle_get_server_info(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(client) = state.clients.get(&connection_id) else {
        send_err(
            evt_tx,
            connection_id,
            BackendOperation::ServerInfo,
            "Not connected",
        )
        .await;
        return;
    };

    let info = client.server_info();
    let _ = evt_tx
        .send(BackendEvent::ServerInfoLoaded {
            connection_id,
            info: ServerInfoSnapshot::from_info(&info),
        })
        .await;
}

pub(crate) async fn handle_get_jetstream_account_info(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(client) = state.clients.get(&connection_id) else {
        send_err(
            evt_tx,
            connection_id,
            BackendOperation::JetStreamAccountInfo,
            "Not connected",
        )
        .await;
        return;
    };

    let js = async_nats::jetstream::new(client.clone());
    match js.query_account().await {
        Ok(account) => {
            let _ = evt_tx
                .send(BackendEvent::JetStreamAccountInfoLoaded {
                    connection_id,
                    info: JetStreamAccountInfoSnapshot::from_account(account),
                })
                .await;
        }
        Err(e) => {
            send_err(
                evt_tx,
                connection_id,
                BackendOperation::JetStreamAccountInfo,
                &e.to_string(),
            )
            .await
        }
    }
}

async fn send_err(
    evt_tx: &mpsc::Sender<BackendEvent>,
    connection_id: u64,
    operation: BackendOperation,
    message: &str,
) {
    let _ = evt_tx
        .send(BackendEvent::Error {
            connection_id: Some(connection_id),
            backend_id: None,
            operation,
            message: message.to_string(),
            context: None,
        })
        .await;
}
