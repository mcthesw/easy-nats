use bytes::Bytes;
use futures_util::TryStreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::cancellation::TaskCancellation;
use crate::event::BackendEvent;

use super::super::helpers::kv_entry_to_json;
use super::super::state::WorkerState;

/// Spawn a task that streams key names in batches.
/// Returns the JoinHandle so the caller can track/cancel it.
pub(crate) fn handle_list_keys(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    cancel: TaskCancellation,
    generation: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) -> Option<JoinHandle<()>> {
    let js = super::jetstream(state, connection_id, evt_tx, "list_kv_keys")?;
    let evt_tx = evt_tx.clone();
    let token = cancel.into_token();

    Some(tokio::spawn(async move {
        let store = match js.get_key_value(&bucket).await {
            Ok(s) => s,
            Err(e) => {
                super::send_err(&evt_tx, connection_id, "list_kv_keys", e.to_string());
                return;
            }
        };

        let mut keys_stream = match store.keys().await {
            Ok(k) => k,
            Err(e) => {
                super::send_err(&evt_tx, connection_id, "list_kv_keys", e.to_string());
                return;
            }
        };

        const BATCH_SIZE: usize = 200;
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        loop {
            let next = tokio::select! {
                _ = token.cancelled() => return,
                result = keys_stream.try_next() => result,
            };

            match next {
                Ok(Some(key)) => {
                    batch.push(key);
                    if batch.len() >= BATCH_SIZE {
                        let entries: Vec<serde_json::Value> = batch
                            .drain(..)
                            .map(|k| serde_json::json!({ "key": k }))
                            .collect();
                        super::send_ok(
                            &evt_tx,
                            connection_id,
                            "list_kv_keys",
                            serde_json::json!({ "bucket": &bucket, "entries": entries, "done": false, "generation": generation }),
                        );
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    super::send_err(&evt_tx, connection_id, "list_kv_keys", e.to_string());
                    return;
                }
            }
        }

        // Send remaining keys
        if !batch.is_empty() {
            let entries: Vec<serde_json::Value> = batch
                .drain(..)
                .map(|k| serde_json::json!({ "key": k }))
                .collect();
            super::send_ok(
                &evt_tx,
                connection_id,
                "list_kv_keys",
                serde_json::json!({ "bucket": &bucket, "entries": entries, "done": false, "generation": generation }),
            );
        }

        // Signal completion
        super::send_ok(
            &evt_tx,
            connection_id,
            "list_kv_keys",
            serde_json::json!({ "bucket": &bucket, "entries": [], "done": true, "generation": generation }),
        );
    }))
}

pub(crate) async fn handle_get_entry(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "get_kv_entry").await
    else {
        return;
    };
    match store.entry(&key).await {
        Ok(entry) => {
            let entry_json = match entry.as_ref().map(kv_entry_to_json) {
                Some(v) => v,
                None => serde_json::json!({ "key": key }),
            };
            super::send_ok(
                evt_tx,
                connection_id,
                "get_kv_entry",
                serde_json::json!({ "bucket": bucket, "entry": entry_json }),
            );
        }
        Err(e) => super::send_err(evt_tx, connection_id, "get_kv_entry", e.to_string()),
    }
}

pub(crate) async fn handle_get_history(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "get_kv_history").await
    else {
        return;
    };
    match store.history(&key).await {
        Ok(mut history) => {
            let mut entries = Vec::new();
            loop {
                match history.try_next().await {
                    Ok(Some(entry)) => entries.push(kv_entry_to_json(&entry)),
                    Ok(None) => break,
                    Err(e) => {
                        super::send_err(evt_tx, connection_id, "get_kv_history", e.to_string());
                        return;
                    }
                }
            }
            entries.sort_by(|a, b| b["revision"].as_u64().cmp(&a["revision"].as_u64()));
            super::send_ok(
                evt_tx,
                connection_id,
                "get_kv_history",
                serde_json::json!({ "bucket": bucket, "key": key, "history": entries }),
            );
        }
        Err(e) => super::send_err(evt_tx, connection_id, "get_kv_history", e.to_string()),
    }
}

pub(crate) async fn handle_put_entry(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    value: Vec<u8>,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "put_kv_entry").await
    else {
        return;
    };
    match store.put(&key, Bytes::from(value)).await {
        Ok(revision) => super::send_ok(
            evt_tx,
            connection_id,
            "put_kv_entry",
            serde_json::json!({ "bucket": bucket, "key": key, "revision": revision }),
        ),
        Err(e) => super::send_err(evt_tx, connection_id, "put_kv_entry", e.to_string()),
    }
}

pub(crate) async fn handle_delete_entry(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "delete_kv_entry").await
    else {
        return;
    };
    match store.delete(&key).await {
        Ok(()) => super::send_ok(
            evt_tx,
            connection_id,
            "delete_kv_entry",
            serde_json::json!({ "bucket": bucket, "key": key }),
        ),
        Err(e) => super::send_err(evt_tx, connection_id, "delete_kv_entry", e.to_string()),
    }
}

pub(crate) async fn handle_purge_entry(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    key: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "purge_kv_entry").await
    else {
        return;
    };
    match store.purge(&key).await {
        Ok(()) => super::send_ok(
            evt_tx,
            connection_id,
            "purge_kv_entry",
            serde_json::json!({ "bucket": bucket, "key": key }),
        ),
        Err(e) => super::send_err(evt_tx, connection_id, "purge_kv_entry", e.to_string()),
    }
}
