use nats_backend::BackendCommand;

use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_obj_store_operation(
        &mut self,
        connection_id: u64,
        operation: &str,
        data: &serde_json::Value,
    ) -> bool {
        match operation {
            "list_object_store_buckets" => {
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
            "create_object_store_bucket" | "delete_object_store_bucket" => {
                self.toasts
                    .push(ToastLevel::Success, format!("{operation} succeeded"));
                self.backend
                    .send(BackendCommand::ListObjectStoreBuckets { connection_id });
                true
            }
            "list_objects" => {
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
            "upload_object" | "delete_object" => {
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
            "download_object" => {
                // Download handled by the tab UI that initiated it
                true
            }
            _ => false,
        }
    }

    pub(crate) fn clear_obj_store_loading_on_error(&mut self, connection_id: u64, operation: &str) {
        if operation == "list_objects" {
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
