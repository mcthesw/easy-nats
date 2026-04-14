mod connection;
mod consumers;
mod helpers;
mod kv;
mod object_store;
mod pubsub;
mod state;
mod streams;

use tokio::sync::mpsc;

use crate::command::BackendCommand;
use crate::event::BackendEvent;
use state::WorkerState;

pub use helpers::{build_header_map, extract_headers};

pub async fn run_worker(
    mut cmd_rx: mpsc::UnboundedReceiver<BackendCommand>,
    evt_tx: mpsc::UnboundedSender<BackendEvent>,
) {
    tracing::info!("Backend worker started");
    let mut state = WorkerState::default();

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            BackendCommand::Connect { config } => {
                connection::handle_connect(&mut state, config, &evt_tx).await;
            }
            BackendCommand::Disconnect { id } => {
                connection::handle_disconnect(&mut state, id, &evt_tx).await;
            }
            BackendCommand::Publish {
                connection_id,
                subject,
                payload,
                headers,
            } => {
                pubsub::handle_publish(&state, connection_id, subject, payload, headers, &evt_tx)
                    .await;
            }
            BackendCommand::Subscribe {
                connection_id,
                subscriber_id,
                subject,
            } => {
                pubsub::handle_subscribe(
                    &mut state,
                    connection_id,
                    subscriber_id,
                    subject,
                    &evt_tx,
                )
                .await;
            }
            BackendCommand::Unsubscribe {
                connection_id,
                subscriber_id,
                subject,
            } => {
                pubsub::handle_unsubscribe(
                    &mut state,
                    connection_id,
                    subscriber_id,
                    subject,
                    &evt_tx,
                )
                .await;
            }
            BackendCommand::Request {
                connection_id,
                subject,
                payload,
                headers,
                timeout_ms,
            } => {
                pubsub::handle_request(
                    &state,
                    connection_id,
                    subject,
                    payload,
                    headers,
                    timeout_ms,
                    &evt_tx,
                )
                .await;
            }
            BackendCommand::ListStreams { connection_id } => {
                streams::handle_list_streams(&state, connection_id, &evt_tx).await;
            }
            BackendCommand::CreateStream {
                connection_id,
                config,
            } => {
                streams::handle_create_stream(&state, connection_id, config, &evt_tx).await;
            }
            BackendCommand::UpdateStream {
                connection_id,
                config,
            } => {
                streams::handle_update_stream(&state, connection_id, config, &evt_tx).await;
            }
            BackendCommand::DeleteStream {
                connection_id,
                name,
            } => {
                streams::handle_delete_stream(&state, connection_id, name, &evt_tx).await;
            }
            BackendCommand::PurgeStream {
                connection_id,
                name,
                filter,
            } => {
                streams::handle_purge_stream(&state, connection_id, name, filter, &evt_tx).await;
            }
            BackendCommand::GetStreamMessages {
                connection_id,
                stream,
                start_sequence,
                subject_filter,
                batch_size,
            } => {
                streams::handle_get_messages(
                    &state,
                    connection_id,
                    stream,
                    start_sequence,
                    subject_filter,
                    batch_size,
                    &evt_tx,
                )
                .await;
            }
            BackendCommand::DeleteStreamMessage {
                connection_id,
                stream,
                sequence,
            } => {
                streams::handle_delete_message(&state, connection_id, stream, sequence, &evt_tx)
                    .await;
            }
            BackendCommand::ListConsumers {
                connection_id,
                stream,
            } => {
                consumers::handle_list_consumers(&state, connection_id, stream, &evt_tx).await;
            }
            BackendCommand::CreateConsumer {
                connection_id,
                stream,
                config,
            } => {
                consumers::handle_create_consumer(&state, connection_id, stream, config, &evt_tx)
                    .await;
            }
            BackendCommand::DeleteConsumer {
                connection_id,
                stream,
                name,
            } => {
                consumers::handle_delete_consumer(&state, connection_id, stream, name, &evt_tx)
                    .await;
            }
            BackendCommand::ListKvBuckets { connection_id } => {
                kv::handle_list_buckets(&state, connection_id, &evt_tx).await;
            }
            BackendCommand::CreateKvBucket {
                connection_id,
                config,
            } => {
                kv::handle_create_bucket(&state, connection_id, config, &evt_tx).await;
            }
            BackendCommand::DeleteKvBucket {
                connection_id,
                bucket,
            } => {
                kv::handle_delete_bucket(&state, connection_id, bucket, &evt_tx).await;
            }
            BackendCommand::ListKvKeys {
                connection_id,
                bucket,
            } => {
                kv::handle_list_keys(&state, connection_id, bucket, &evt_tx).await;
            }
            BackendCommand::GetKvEntry {
                connection_id,
                bucket,
                key,
            } => {
                kv::handle_get_entry(&state, connection_id, bucket, key, &evt_tx).await;
            }
            BackendCommand::PutKvEntry {
                connection_id,
                bucket,
                key,
                value,
            } => {
                kv::handle_put_entry(&state, connection_id, bucket, key, value, &evt_tx).await;
            }
            BackendCommand::DeleteKvEntry {
                connection_id,
                bucket,
                key,
            } => {
                kv::handle_delete_entry(&state, connection_id, bucket, key, &evt_tx).await;
            }
            BackendCommand::PurgeKvEntry {
                connection_id,
                bucket,
                key,
            } => {
                kv::handle_purge_entry(&state, connection_id, bucket, key, &evt_tx).await;
            }
            BackendCommand::GetKvHistory {
                connection_id,
                bucket,
                key,
            } => {
                kv::handle_get_history(&state, connection_id, bucket, key, &evt_tx).await;
            }
            BackendCommand::ListObjectStoreBuckets { connection_id } => {
                object_store::handle_list_buckets(&state, connection_id, &evt_tx).await;
            }
            BackendCommand::CreateObjectStoreBucket {
                connection_id,
                config,
            } => {
                object_store::handle_create_bucket(&state, connection_id, config, &evt_tx).await;
            }
            BackendCommand::DeleteObjectStoreBucket {
                connection_id,
                bucket,
            } => {
                object_store::handle_delete_bucket(&state, connection_id, bucket, &evt_tx).await;
            }
            BackendCommand::ListObjects {
                connection_id,
                bucket,
            } => {
                object_store::handle_list_objects(&state, connection_id, bucket, &evt_tx).await;
            }
            BackendCommand::DownloadObject {
                connection_id,
                bucket,
                name,
                file_path,
            } => {
                object_store::handle_download_object(
                    &state,
                    connection_id,
                    bucket,
                    name,
                    file_path,
                    &evt_tx,
                )
                .await;
            }
            BackendCommand::DeleteObject {
                connection_id,
                bucket,
                name,
            } => {
                object_store::handle_delete_object(&state, connection_id, bucket, name, &evt_tx)
                    .await;
            }
            BackendCommand::UploadObject {
                connection_id,
                bucket,
                name,
                data,
            } => {
                object_store::handle_upload_object(
                    &state,
                    connection_id,
                    bucket,
                    name,
                    data,
                    &evt_tx,
                )
                .await;
            }
        }
    }

    tracing::info!("Backend worker stopped");
}
