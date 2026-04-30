use bytes::Bytes;
use futures_util::TryStreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::super::state::WorkerState;
use crate::cancellation::TaskCancellation;
use crate::event::{BackendEvent, BackendOperation};
use crate::models::{BackendErrorContext, KvEntryInfo, KvHistoryItem, KvKeyBatch};

/// Spawn a task that streams key names in batches.
/// Returns the JoinHandle so the caller can track/cancel it.
pub(crate) async fn handle_list_keys(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    cancel: TaskCancellation,
    generation: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) -> Option<JoinHandle<()>> {
    let js =
        match super::jetstream(state, connection_id, evt_tx, BackendOperation::ListKvKeys).await {
            Some(js) => js,
            None => return None,
        };
    tracing::info!(connection_id, bucket = %bucket, generation, "Starting KV key list task");
    let evt_tx = evt_tx.clone();
    let token = cancel.into_token();

    Some(tokio::spawn(async move {
        let mut total_keys = 0usize;
        let store = match js.get_key_value(&bucket).await {
            Ok(s) => s,
            Err(e) => {
                super::send_err(
                    &evt_tx,
                    connection_id,
                    BackendOperation::ListKvKeys,
                    e.to_string(),
                )
                .await;
                return;
            }
        };

        let mut keys_stream = match store.keys().await {
            Ok(k) => k,
            Err(e) => {
                super::send_err(
                    &evt_tx,
                    connection_id,
                    BackendOperation::ListKvKeys,
                    e.to_string(),
                )
                .await;
                return;
            }
        };

        const BATCH_SIZE: usize = 200;
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        loop {
            let next = tokio::select! {
                _ = token.cancelled() => {
                    tracing::debug!(connection_id, bucket = %bucket, generation, total_keys, "Cancelled KV key list task");
                    return;
                },
                result = keys_stream.try_next() => result,
            };

            match next {
                Ok(Some(key)) => {
                    batch.push(key);
                    if batch.len() >= BATCH_SIZE {
                        let keys = std::mem::take(&mut batch);
                        total_keys += keys.len();
                        tracing::debug!(connection_id, bucket = %bucket, generation, batch_size = keys.len(), total_keys, "Sending KV key batch");
                        let _ = evt_tx
                            .send(BackendEvent::KvKeysListed {
                                connection_id,
                                batch: KvKeyBatch {
                                    bucket: bucket.clone(),
                                    keys,
                                    done: false,
                                    generation,
                                },
                            })
                            .await;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    super::send_err(
                        &evt_tx,
                        connection_id,
                        BackendOperation::ListKvKeys,
                        e.to_string(),
                    )
                    .await;
                    return;
                }
            }
        }

        // Send remaining keys
        if !batch.is_empty() {
            let keys = std::mem::take(&mut batch);
            total_keys += keys.len();
            tracing::debug!(connection_id, bucket = %bucket, generation, batch_size = keys.len(), total_keys, "Sending final KV key batch");
            let _ = evt_tx
                .send(BackendEvent::KvKeysListed {
                    connection_id,
                    batch: KvKeyBatch {
                        bucket: bucket.clone(),
                        keys,
                        done: false,
                        generation,
                    },
                })
                .await;
        }

        tracing::info!(connection_id, bucket = %bucket, generation, total_keys, "Completed KV key list task");

        // Signal completion
        let _ = evt_tx
            .send(BackendEvent::KvKeysListed {
                connection_id,
                batch: KvKeyBatch {
                    bucket,
                    keys: Vec::new(),
                    done: true,
                    generation,
                },
            })
            .await;
    }))
}

pub(crate) async fn handle_get_entry(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    tracing::debug!(connection_id, bucket = %bucket, key = %key, "Fetching KV entry");
    let error_context = BackendErrorContext::KvEntry {
        bucket: bucket.clone(),
        key: key.clone(),
    };
    let Some(store) = super::open_store_with_context(
        state,
        connection_id,
        &bucket,
        evt_tx,
        BackendOperation::GetKvEntry,
        Some(error_context.clone()),
    )
    .await
    else {
        return;
    };
    match store.entry(&key).await {
        Ok(entry) => {
            tracing::debug!(connection_id, bucket = %bucket, key = %key, found = entry.is_some(), "Fetched KV entry");
            let entry = entry
                .as_ref()
                .map(KvEntryInfo::from_entry)
                .unwrap_or_else(|| KvEntryInfo::missing(bucket, key));
            let _ = evt_tx
                .send(BackendEvent::KvEntryFetched {
                    connection_id,
                    entry,
                })
                .await;
        }
        Err(e) => {
            super::send_err_with_context(
                evt_tx,
                connection_id,
                BackendOperation::GetKvEntry,
                e.to_string(),
                Some(error_context),
            )
            .await
        }
    }
}

pub(crate) async fn handle_get_history(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    tracing::debug!(connection_id, bucket = %bucket, key = %key, "Fetching KV entry");
    let Some(store) = super::open_store(
        state,
        connection_id,
        &bucket,
        evt_tx,
        BackendOperation::GetKvHistory,
    )
    .await
    else {
        return;
    };
    match store.history(&key).await {
        Ok(mut history) => {
            let mut entries = Vec::new();
            loop {
                match history.try_next().await {
                    Ok(Some(entry)) => entries.push(KvHistoryItem::from_entry(&entry)),
                    Ok(None) => break,
                    Err(e) => {
                        super::send_err(
                            evt_tx,
                            connection_id,
                            BackendOperation::GetKvHistory,
                            e.to_string(),
                        )
                        .await;
                        return;
                    }
                }
            }
            entries.sort_by_key(|entry| std::cmp::Reverse(entry.revision));
            let _ = evt_tx
                .send(BackendEvent::KvHistoryFetched {
                    connection_id,
                    bucket,
                    key,
                    history: entries,
                })
                .await;
        }
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::GetKvHistory,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) async fn handle_put_entry(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    value: Vec<u8>,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    tracing::debug!(connection_id, bucket = %bucket, key = %key, "Fetching KV entry");
    let Some(store) = super::open_store(
        state,
        connection_id,
        &bucket,
        evt_tx,
        BackendOperation::PutKvEntry,
    )
    .await
    else {
        return;
    };
    match store.put(&key, Bytes::from(value)).await {
        Ok(_revision) => {
            let _ = evt_tx
                .send(BackendEvent::KvEntryMutated {
                    connection_id,
                    operation: BackendOperation::PutKvEntry,
                    bucket,
                    key,
                })
                .await;
        }
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::PutKvEntry,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) async fn handle_delete_entry(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    tracing::debug!(connection_id, bucket = %bucket, key = %key, "Fetching KV entry");
    let Some(store) = super::open_store(
        state,
        connection_id,
        &bucket,
        evt_tx,
        BackendOperation::DeleteKvEntry,
    )
    .await
    else {
        return;
    };
    match store.delete(&key).await {
        Ok(()) => {
            let _ = evt_tx
                .send(BackendEvent::KvEntryMutated {
                    connection_id,
                    operation: BackendOperation::DeleteKvEntry,
                    bucket,
                    key,
                })
                .await;
        }
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::DeleteKvEntry,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) async fn handle_purge_entry(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    tracing::debug!(connection_id, bucket = %bucket, key = %key, "Fetching KV entry");
    let Some(store) = super::open_store(
        state,
        connection_id,
        &bucket,
        evt_tx,
        BackendOperation::PurgeKvEntry,
    )
    .await
    else {
        return;
    };
    match store.purge(&key).await {
        Ok(()) => {
            let _ = evt_tx
                .send(BackendEvent::KvEntryMutated {
                    connection_id,
                    operation: BackendOperation::PurgeKvEntry,
                    bucket,
                    key,
                })
                .await;
        }
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::PurgeKvEntry,
                e.to_string(),
            )
            .await
        }
    }
}
