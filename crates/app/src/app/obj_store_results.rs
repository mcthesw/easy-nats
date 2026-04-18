use nats_backend::BackendCommand;

use nats_backend::BackendOperation;

use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_obj_store_operation(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        data: &serde_json::Value,
    ) -> bool {
        match operation {
            BackendOperation::ListObjectStoreBuckets => {
                if let Some(arr) = data.as_array() {
                    let infos = arr.clone();
                    for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                        if let TabKind::ObjectStoreBucket {
                            connection_id: cid,
                            bucket_name,
                            state,
                            ..
                        } = tab
                            && *cid == connection_id
                        {
                            state.info = infos
                                .iter()
                                .find(|i| i["bucket"].as_str() == Some(bucket_name.as_str()))
                                .cloned();
                        }
                    }
                    self.obj_store_lists.insert(connection_id, infos);
                }
                true
            }
            BackendOperation::CreateObjectStoreBucket
            | BackendOperation::DeleteObjectStoreBucket => {
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                if operation == BackendOperation::DeleteObjectStoreBucket
                    && let Some(bucket) = data["bucket"].as_str()
                {
                    self.remove_tabs_matching(|tab| {
                        matches!(tab, TabKind::ObjectStoreBucket { connection_id: cid, bucket_name, .. }
                            if *cid == connection_id && bucket_name == bucket)
                    });
                }
                self.backend
                    .send(BackendCommand::ListObjectStoreBuckets { connection_id });
                true
            }
            BackendOperation::ListObjects => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                let objects = data["objects"].as_array().cloned().unwrap_or_default();
                for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                    if let TabKind::ObjectStoreBucket {
                        connection_id: cid,
                        bucket_name,
                        state,
                        ..
                    } = tab
                        && *cid == connection_id
                        && *bucket_name == bucket
                    {
                        state.objects = objects.clone();
                        state.loading_objects = false;
                    }
                }
                true
            }
            BackendOperation::UploadObject | BackendOperation::DeleteObject => {
                let bucket = data["bucket"].as_str().unwrap_or("").to_string();
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                if !bucket.is_empty() {
                    for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                        if let TabKind::ObjectStoreBucket {
                            connection_id: cid,
                            bucket_name,
                            state,
                            ..
                        } = tab
                            && *cid == connection_id
                            && *bucket_name == bucket
                        {
                            state.loading_objects = true;
                        }
                    }
                    self.backend.send(BackendCommand::ListObjects {
                        connection_id,
                        bucket,
                    });
                }
                true
            }
            BackendOperation::DownloadObject => {
                let name = data["name"].as_str().unwrap_or("?");
                let file_path = data["file_path"].as_str().unwrap_or("?");
                self.toasts
                    .push(ToastLevel::Success, format!("{name} → {file_path}"));
                true
            }
            _ => false,
        }
    }

    pub(crate) fn clear_obj_store_loading_on_error(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
    ) {
        if operation == BackendOperation::ListObjects {
            for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
                if let TabKind::ObjectStoreBucket {
                    connection_id: tab_cid,
                    state,
                    ..
                } = tab
                    && *tab_cid == connection_id
                {
                    state.loading_objects = false;
                }
            }
        }
    }
}
