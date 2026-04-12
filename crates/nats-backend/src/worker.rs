use tokio::sync::mpsc;

use crate::command::BackendCommand;
use crate::event::BackendEvent;

/// Main async worker loop that receives commands and dispatches to async-nats.
pub async fn run_worker(
    mut cmd_rx: mpsc::UnboundedReceiver<BackendCommand>,
    evt_tx: mpsc::UnboundedSender<BackendEvent>,
) {
    tracing::info!("Backend worker started");

    while let Some(cmd) = cmd_rx.recv().await {
        tracing::debug!(?cmd, "Received command");

        match cmd {
            BackendCommand::Connect { id } => {
                // TODO: Implement connection logic
                let _ = evt_tx.send(BackendEvent::ConnectionStatus {
                    connection_id: id,
                    status: crate::event::ConnectionStatusKind::Disconnected,
                });
            }
            BackendCommand::Disconnect { id } => {
                let _ = evt_tx.send(BackendEvent::ConnectionStatus {
                    connection_id: id,
                    status: crate::event::ConnectionStatusKind::Disconnected,
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
