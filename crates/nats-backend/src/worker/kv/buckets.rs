use futures_util::TryStreamExt;
use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation};
use crate::models::{KvBucketConfigInput, KvBucketInfo};

use super::super::state::WorkerState;

pub(crate) async fn handle_list_buckets(
    state: &WorkerState,
    connection_id: u64,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(
        state,
        connection_id,
        evt_tx,
        BackendOperation::ListKvBuckets,
    )
    .await
    else {
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
                        Ok(status) => buckets.push(KvBucketInfo::from_status(&status)),
                        Err(e) => {
                            tracing::warn!(bucket = bucket_name, %e, "Error loading KV status")
                        }
                    },
                    Err(e) => tracing::warn!(bucket = bucket_name, %e, "Error opening KV bucket"),
                }
            }
            Ok(None) => break,
            Err(e) => {
                super::send_err(
                    evt_tx,
                    connection_id,
                    BackendOperation::ListKvBuckets,
                    e.to_string(),
                )
                .await;
                return;
            }
        }
    }
    buckets.sort_by(|a, b| a.bucket.cmp(&b.bucket));
    let _ = evt_tx
        .send(BackendEvent::KvBucketsListed {
            connection_id,
            buckets,
        })
        .await;
}

pub(crate) async fn handle_create_bucket(
    state: &WorkerState,
    connection_id: u64,
    config: KvBucketConfigInput,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(
        state,
        connection_id,
        evt_tx,
        BackendOperation::CreateKvBucket,
    )
    .await
    else {
        return;
    };

    if config.bucket.trim().is_empty() {
        super::send_err(
            evt_tx,
            connection_id,
            BackendOperation::CreateKvBucket,
            "Bucket name is required".to_string(),
        )
        .await;
        return;
    }

    match js.create_key_value(config.into_async_nats()).await {
        Ok(store) => match store.status().await {
            Ok(status) => {
                let _ = evt_tx
                    .send(BackendEvent::KvBucketCreated {
                        connection_id,
                        bucket: KvBucketInfo::from_status(&status),
                    })
                    .await;
            }
            Err(e) => {
                super::send_err(
                    evt_tx,
                    connection_id,
                    BackendOperation::CreateKvBucket,
                    e.to_string(),
                )
                .await
            }
        },
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::CreateKvBucket,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) async fn handle_delete_bucket(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(
        state,
        connection_id,
        evt_tx,
        BackendOperation::DeleteKvBucket,
    )
    .await
    else {
        return;
    };

    match js.delete_key_value(&bucket).await {
        Ok(_) => {
            let _ = evt_tx
                .send(BackendEvent::KvBucketDeleted {
                    connection_id,
                    bucket,
                })
                .await;
        }
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::DeleteKvBucket,
                e.to_string(),
            )
            .await
        }
    }
}

pub(crate) async fn handle_update_bucket(
    state: &WorkerState,
    connection_id: u64,
    config: KvBucketConfigInput,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(
        state,
        connection_id,
        evt_tx,
        BackendOperation::UpdateKvBucket,
    )
    .await
    else {
        return;
    };

    if config.bucket.trim().is_empty() {
        super::send_err(
            evt_tx,
            connection_id,
            BackendOperation::UpdateKvBucket,
            "Bucket name is required".to_string(),
        )
        .await;
        return;
    }

    match js.update_key_value(config.into_async_nats()).await {
        Ok(store) => match store.status().await {
            Ok(status) => {
                let _ = evt_tx
                    .send(BackendEvent::KvBucketUpdated {
                        connection_id,
                        bucket: KvBucketInfo::from_status(&status),
                    })
                    .await;
            }
            Err(e) => {
                super::send_err(
                    evt_tx,
                    connection_id,
                    BackendOperation::UpdateKvBucket,
                    e.to_string(),
                )
                .await
            }
        },
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::UpdateKvBucket,
                e.to_string(),
            )
            .await
        }
    }
}
