use bytes::Bytes;
use futures_util::TryStreamExt;
use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::super::helpers::kv_entry_to_json;
use super::super::state::WorkerState;

pub(crate) async fn handle_list_keys(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "list_kv_keys").await
    else {
        return;
    };

    match store.keys().await {
        Ok(mut keys) => {
            let mut entries = Vec::new();
            loop {
                match keys.try_next().await {
                    Ok(Some(key)) => match store.entry(key.clone()).await {
                        Ok(Some(entry)) => entries.push(kv_entry_to_json(&entry)),
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!(bucket = %bucket, key = %key, %e, "Error loading KV entry")
                        }
                    },
                    Ok(None) => break,
                    Err(e) => {
                        super::send_err(evt_tx, connection_id, "list_kv_keys", e.to_string());
                        return;
                    }
                }
            }
            entries.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
            super::send_ok(
                evt_tx,
                connection_id,
                "list_kv_keys",
                serde_json::json!({ "bucket": bucket, "entries": entries }),
            );
        }
        Err(e) => super::send_err(evt_tx, connection_id, "list_kv_keys", e.to_string()),
    }
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
        Ok(entry) => super::send_ok(
            evt_tx,
            connection_id,
            "get_kv_entry",
            serde_json::json!({ "bucket": bucket, "entry": entry.as_ref().map(kv_entry_to_json) }),
        ),
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
