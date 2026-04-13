use futures_util::TryStreamExt;
use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::super::state::WorkerState;

pub(crate) async fn handle_list_buckets(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(js) = super::jetstream(state, connection_id, evt_tx, "list_object_store_buckets")
    else {
        return;
    };

    let mut stream_names = js.stream_names();
    let mut buckets = Vec::new();
    loop {
        match stream_names.try_next().await {
            Ok(Some(stream_name)) => {
                let Some(bucket_name) = stream_name.strip_prefix("OBJ_") else {
                    continue;
                };
                match js.get_stream(&stream_name).await {
                    Ok(mut stream) => match stream.info().await {
                        Ok(info) => {
                            buckets.push(serde_json::json!({
                                "bucket": bucket_name,
                                "description": info.config.description.as_deref().unwrap_or(""),
                                "storage": format!("{:?}", info.config.storage),
                                "bytes": info.state.bytes,
                                "max_bytes": info.config.max_bytes,
                                "messages": info.state.messages,
                            }));
                        }
                        Err(e) => {
                            tracing::warn!(bucket = bucket_name, %e, "Error loading object store info")
                        }
                    },
                    Err(e) => {
                        tracing::warn!(bucket = bucket_name, %e, "Error opening object store stream")
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                super::send_err(
                    evt_tx,
                    connection_id,
                    "list_object_store_buckets",
                    e.to_string(),
                );
                return;
            }
        }
    }

    buckets.sort_by(|a, b| a["bucket"].as_str().cmp(&b["bucket"].as_str()));
    super::send_ok(
        evt_tx,
        connection_id,
        "list_object_store_buckets",
        serde_json::Value::Array(buckets),
    );
}

pub(crate) async fn handle_create_bucket(
    state: &WorkerState,
    connection_id: u64,
    config: serde_json::Value,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(js) = super::jetstream(state, connection_id, evt_tx, "create_object_store_bucket")
    else {
        return;
    };

    let bucket = config["bucket"].as_str().unwrap_or("").trim().to_string();
    if bucket.is_empty() {
        super::send_err(
            evt_tx,
            connection_id,
            "create_object_store_bucket",
            "Bucket name is required".to_string(),
        );
        return;
    }

    let storage = match config["storage"].as_str() {
        Some("memory") | Some("Memory") => async_nats::jetstream::stream::StorageType::Memory,
        _ => async_nats::jetstream::stream::StorageType::File,
    };

    let bucket_config = async_nats::jetstream::object_store::Config {
        bucket: bucket.clone(),
        description: config["description"].as_str().map(|s| s.to_string()),
        max_bytes: config["max_bytes"].as_i64().unwrap_or_default(),
        storage,
        num_replicas: config["num_replicas"].as_u64().unwrap_or(1) as usize,
        ..Default::default()
    };

    match js.create_object_store(bucket_config).await {
        Ok(_store) => super::send_ok(
            evt_tx,
            connection_id,
            "create_object_store_bucket",
            serde_json::json!({ "bucket": bucket }),
        ),
        Err(e) => super::send_err(
            evt_tx,
            connection_id,
            "create_object_store_bucket",
            e.to_string(),
        ),
    }
}

pub(crate) async fn handle_delete_bucket(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(js) = super::jetstream(state, connection_id, evt_tx, "delete_object_store_bucket")
    else {
        return;
    };

    match js.delete_object_store(&bucket).await {
        Ok(_) => super::send_ok(
            evt_tx,
            connection_id,
            "delete_object_store_bucket",
            serde_json::json!({ "bucket": bucket }),
        ),
        Err(e) => super::send_err(
            evt_tx,
            connection_id,
            "delete_object_store_bucket",
            e.to_string(),
        ),
    }
}
