use futures_util::TryStreamExt;
use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::super::helpers::kv_status_to_json;
use super::super::state::WorkerState;

pub(crate) async fn handle_list_buckets(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(state, connection_id, evt_tx, "list_kv_buckets").await else {
        return;
    };

    let mut stream_names = js.stream_names();
    let mut buckets = Vec::new();
    loop {
        match stream_names.try_next().await {
            Ok(Some(stream_name)) => {
                let Some(bucket_name) = stream_name.strip_prefix("KV_") else {
                    continue;
                };
                match js.get_key_value(bucket_name).await {
                    Ok(store) => match store.status().await {
                        Ok(status) => buckets.push(kv_status_to_json(&status)),
                        Err(e) => {
                            tracing::warn!(bucket = bucket_name, %e, "Error loading KV status")
                        }
                    },
                    Err(e) => tracing::warn!(bucket = bucket_name, %e, "Error opening KV bucket"),
                }
            }
            Ok(None) => break,
            Err(e) => {
                super::send_err(evt_tx, connection_id, "list_kv_buckets", e.to_string()).await;
                return;
            }
        }
    }
    buckets.sort_by(|a, b| a["bucket"].as_str().cmp(&b["bucket"].as_str()));
    super::send_ok(
        evt_tx,
        connection_id,
        "list_kv_buckets",
        serde_json::Value::Array(buckets),
    )
    .await;
}

pub(crate) async fn handle_create_bucket(
    state: &WorkerState,
    connection_id: u64,
    config: serde_json::Value,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(state, connection_id, evt_tx, "create_kv_bucket").await else {
        return;
    };

    let bucket = config["bucket"].as_str().unwrap_or("").trim().to_string();
    if bucket.is_empty() {
        super::send_err(
            evt_tx,
            connection_id,
            "create_kv_bucket",
            "Bucket name is required".to_string(),
        )
        .await;
        return;
    }

    let storage = match config["storage"].as_str() {
        Some("memory") | Some("Memory") => async_nats::jetstream::stream::StorageType::Memory,
        _ => async_nats::jetstream::stream::StorageType::File,
    };

    let mut bucket_config = async_nats::jetstream::kv::Config {
        bucket,
        history: config["history"].as_i64().unwrap_or(1),
        storage,
        description: config["description"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        max_value_size: config["max_value_size"].as_i64().unwrap_or_default() as i32,
        max_bytes: config["max_bytes"].as_i64().unwrap_or_default(),
        num_replicas: config["num_replicas"].as_u64().unwrap_or(1) as usize,
        ..Default::default()
    };
    if let Some(max_age) = config["max_age"].as_u64() {
        bucket_config.max_age = std::time::Duration::from_nanos(max_age);
    }

    match js.create_key_value(bucket_config).await {
        Ok(store) => match store.status().await {
            Ok(status) => {
                super::send_ok(
                    evt_tx,
                    connection_id,
                    "create_kv_bucket",
                    kv_status_to_json(&status),
                )
                .await
            }
            Err(e) => {
                super::send_err(evt_tx, connection_id, "create_kv_bucket", e.to_string()).await
            }
        },
        Err(e) => super::send_err(evt_tx, connection_id, "create_kv_bucket", e.to_string()).await,
    }
}

pub(crate) async fn handle_delete_bucket(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(state, connection_id, evt_tx, "delete_kv_bucket").await else {
        return;
    };

    match js.delete_key_value(&bucket).await {
        Ok(_) => {
            super::send_ok(
                evt_tx,
                connection_id,
                "delete_kv_bucket",
                serde_json::json!({ "bucket": bucket }),
            )
            .await
        }
        Err(e) => super::send_err(evt_tx, connection_id, "delete_kv_bucket", e.to_string()).await,
    }
}

pub(crate) async fn handle_update_bucket(
    state: &WorkerState,
    connection_id: u64,
    config: serde_json::Value,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(state, connection_id, evt_tx, "update_kv_bucket").await else {
        return;
    };

    let bucket = config["bucket"].as_str().unwrap_or("").trim().to_string();
    if bucket.is_empty() {
        super::send_err(
            evt_tx,
            connection_id,
            "update_kv_bucket",
            "Bucket name is required".to_string(),
        )
        .await;
        return;
    }

    let storage = match config["storage"].as_str() {
        Some("memory") | Some("Memory") => async_nats::jetstream::stream::StorageType::Memory,
        _ => async_nats::jetstream::stream::StorageType::File,
    };

    let mut bucket_config = async_nats::jetstream::kv::Config {
        bucket,
        history: config["history"].as_i64().unwrap_or(1),
        storage,
        description: config["description"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        max_value_size: config["max_value_size"].as_i64().unwrap_or_default() as i32,
        max_bytes: config["max_bytes"].as_i64().unwrap_or_default(),
        num_replicas: config["num_replicas"].as_u64().unwrap_or(1) as usize,
        ..Default::default()
    };
    if let Some(max_age) = config["max_age"].as_u64() {
        bucket_config.max_age = std::time::Duration::from_nanos(max_age);
    }

    match js.update_key_value(bucket_config).await {
        Ok(store) => match store.status().await {
            Ok(status) => {
                super::send_ok(
                    evt_tx,
                    connection_id,
                    "update_kv_bucket",
                    kv_status_to_json(&status),
                )
                .await
            }
            Err(e) => {
                super::send_err(evt_tx, connection_id, "update_kv_bucket", e.to_string()).await
            }
        },
        Err(e) => super::send_err(evt_tx, connection_id, "update_kv_bucket", e.to_string()).await,
    }
}
