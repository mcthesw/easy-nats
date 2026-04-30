use futures_util::TryStreamExt;
use tokio::sync::mpsc;

use crate::event::{BackendEvent, BackendOperation};
use crate::models::{ObjectStoreBucketConfigInput, ObjectStoreBucketInfo};

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
        BackendOperation::ListObjectStoreBuckets,
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
                let Some(bucket_name) = stream_name.strip_prefix("OBJ_") else {
                    continue;
                };
                match js.get_stream(&stream_name).await {
                    Ok(mut stream) => match stream.info().await {
                        Ok(info) => {
                            buckets.push(ObjectStoreBucketInfo::from_stream_info(
                                bucket_name.to_string(),
                                info,
                            ));
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
                    BackendOperation::ListObjectStoreBuckets,
                    e.to_string(),
                )
                .await;
                return;
            }
        }
    }

    buckets.sort_by(|a, b| a.bucket.cmp(&b.bucket));
    let _ = evt_tx
        .send(BackendEvent::ObjectStoreBucketsListed {
            connection_id,
            buckets,
        })
        .await;
}

pub(crate) async fn handle_create_bucket(
    state: &WorkerState,
    connection_id: u64,
    config: ObjectStoreBucketConfigInput,
    evt_tx: &mpsc::Sender<BackendEvent>,
) {
    let Some(js) = super::jetstream(
        state,
        connection_id,
        evt_tx,
        BackendOperation::CreateObjectStoreBucket,
    )
    .await
    else {
        return;
    };

    let bucket = config.bucket.trim().to_string();
    if bucket.is_empty() {
        super::send_err(
            evt_tx,
            connection_id,
            BackendOperation::CreateObjectStoreBucket,
            "Bucket name is required".to_string(),
        )
        .await;
        return;
    }

    match js.create_object_store(config.into_async_nats()).await {
        Ok(_store) => match load_bucket_info(&js, &bucket).await {
            Ok(info) => {
                let _ = evt_tx
                    .send(BackendEvent::ObjectStoreBucketCreated {
                        connection_id,
                        bucket: info,
                    })
                    .await;
            }
            Err(e) => {
                super::send_err(
                    evt_tx,
                    connection_id,
                    BackendOperation::CreateObjectStoreBucket,
                    e,
                )
                .await
            }
        },
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::CreateObjectStoreBucket,
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
        BackendOperation::DeleteObjectStoreBucket,
    )
    .await
    else {
        return;
    };

    match js.delete_object_store(&bucket).await {
        Ok(_) => {
            let _ = evt_tx
                .send(BackendEvent::ObjectStoreBucketDeleted {
                    connection_id,
                    bucket,
                })
                .await;
        }
        Err(e) => {
            super::send_err(
                evt_tx,
                connection_id,
                BackendOperation::DeleteObjectStoreBucket,
                e.to_string(),
            )
            .await
        }
    }
}

async fn load_bucket_info(
    js: &async_nats::jetstream::Context,
    bucket: &str,
) -> Result<ObjectStoreBucketInfo, String> {
    let stream_name = format!("OBJ_{bucket}");
    let mut stream = js
        .get_stream(&stream_name)
        .await
        .map_err(|e| e.to_string())?;
    let info = stream.info().await.map_err(|e| e.to_string())?;
    Ok(ObjectStoreBucketInfo::from_stream_info(
        bucket.to_string(),
        info,
    ))
}
