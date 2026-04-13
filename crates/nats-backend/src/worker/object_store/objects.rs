use base64::Engine;
use futures_util::TryStreamExt;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

use crate::event::BackendEvent;

use super::super::state::WorkerState;

pub(crate) async fn handle_list_objects(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "list_objects").await
    else {
        return;
    };

    let list_result = store.list().await;
    let mut items = match list_result {
        Ok(list) => list,
        Err(e) => {
            super::send_err(evt_tx, connection_id, "list_objects", e.to_string());
            return;
        }
    };

    let mut objects = Vec::new();
    loop {
        match items.try_next().await {
            Ok(Some(info)) => {
                if info.deleted {
                    continue;
                }
                objects.push(obj_info_to_json(&info));
            }
            Ok(None) => break,
            Err(e) => {
                super::send_err(evt_tx, connection_id, "list_objects", e.to_string());
                return;
            }
        }
    }

    objects.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    super::send_ok(
        evt_tx,
        connection_id,
        "list_objects",
        serde_json::json!({ "bucket": bucket, "objects": objects }),
    );
}

pub(crate) async fn handle_upload_object(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    name: String,
    data: Vec<u8>,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "upload_object").await
    else {
        return;
    };

    let mut cursor = tokio::io::BufReader::new(std::io::Cursor::new(data));
    match store.put(name.as_str(), &mut cursor).await {
        Ok(info) => super::send_ok(
            evt_tx,
            connection_id,
            "upload_object",
            obj_info_to_json(&info),
        ),
        Err(e) => super::send_err(evt_tx, connection_id, "upload_object", e.to_string()),
    }
}

pub(crate) async fn handle_download_object(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    name: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "download_object").await
    else {
        return;
    };

    let mut object = match store.get(&name).await {
        Ok(obj) => obj,
        Err(e) => {
            super::send_err(evt_tx, connection_id, "download_object", e.to_string());
            return;
        }
    };

    let mut buf = Vec::new();
    match object.read_to_end(&mut buf).await {
        Ok(_) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
            super::send_ok(
                evt_tx,
                connection_id,
                "download_object",
                serde_json::json!({
                    "bucket": bucket,
                    "name": name,
                    "data_base64": b64,
                    "size": buf.len(),
                }),
            );
        }
        Err(e) => super::send_err(evt_tx, connection_id, "download_object", e.to_string()),
    }
}

pub(crate) async fn handle_delete_object(
    state: &WorkerState,
    connection_id: u64,
    bucket: String,
    name: String,
    evt_tx: &mpsc::UnboundedSender<BackendEvent>,
) {
    let Some(store) =
        super::open_store(state, connection_id, &bucket, evt_tx, "delete_object").await
    else {
        return;
    };

    match store.delete(&name).await {
        Ok(_) => super::send_ok(
            evt_tx,
            connection_id,
            "delete_object",
            serde_json::json!({ "bucket": bucket, "name": name }),
        ),
        Err(e) => super::send_err(evt_tx, connection_id, "delete_object", e.to_string()),
    }
}

fn obj_info_to_json(info: &async_nats::jetstream::object_store::ObjectInfo) -> serde_json::Value {
    serde_json::json!({
        "name": info.name,
        "description": info.description,
        "size": info.size,
        "chunks": info.chunks,
        "modified": info.modified.map(|t| t.to_string()),
        "digest": info.digest,
        "bucket": info.bucket,
    })
}
