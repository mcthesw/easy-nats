use nats_backend::BackendCommand;
use nats_backend::{
    BackendOperation, ObjectStoreBucketInfo, ObjectStoreDownloadResult, ObjectStoreObjectInfo,
};

use crate::tabs::TabKind;
use crate::toast::ToastLevel;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn apply_obj_store_buckets(
        &mut self,
        connection_id: u64,
        buckets: Vec<ObjectStoreBucketInfo>,
    ) {
        for (_surface, tab) in self.dock_state.iter_all_tabs_mut() {
            if let TabKind::ObjectStoreBucket {
                connection_id: cid,
                bucket_name,
                state,
                ..
            } = tab
                && *cid == connection_id
            {
                state.info = buckets
                    .iter()
                    .find(|info| info.bucket == *bucket_name)
                    .cloned();
            }
        }
        self.obj_store_lists.insert(connection_id, buckets);
    }

    pub(crate) fn apply_obj_store_bucket_created(
        &mut self,
        connection_id: u64,
        bucket: ObjectStoreBucketInfo,
    ) {
        self.toasts.push(
            ToastLevel::Success,
            format!("{} succeeded", BackendOperation::CreateObjectStoreBucket),
        );
        upsert_obj_store_bucket(
            self.obj_store_lists.entry(connection_id).or_default(),
            bucket,
        );
        self.backend
            .send(BackendCommand::ListObjectStoreBuckets { connection_id });
    }

    pub(crate) fn apply_obj_store_bucket_deleted(&mut self, connection_id: u64, bucket: String) {
        self.toasts.push(
            ToastLevel::Success,
            format!("{} succeeded", BackendOperation::DeleteObjectStoreBucket),
        );
        if let Some(buckets) = self.obj_store_lists.get_mut(&connection_id) {
            buckets.retain(|info| info.bucket != bucket);
        }
        self.remove_tabs_matching(|tab| {
            matches!(tab, TabKind::ObjectStoreBucket { connection_id: cid, bucket_name, .. }
                if *cid == connection_id && bucket_name == &bucket)
        });
        self.backend
            .send(BackendCommand::ListObjectStoreBuckets { connection_id });
    }

    pub(crate) fn apply_obj_store_objects(
        &mut self,
        connection_id: u64,
        bucket: String,
        objects: Vec<ObjectStoreObjectInfo>,
    ) {
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
    }

    pub(crate) fn apply_obj_store_uploaded(
        &mut self,
        connection_id: u64,
        object: ObjectStoreObjectInfo,
    ) {
        self.refresh_object_store_after_mutation(
            connection_id,
            BackendOperation::UploadObject,
            object.bucket,
        );
    }

    pub(crate) fn apply_obj_store_deleted(
        &mut self,
        connection_id: u64,
        bucket: String,
        _name: String,
    ) {
        self.refresh_object_store_after_mutation(
            connection_id,
            BackendOperation::DeleteObject,
            bucket,
        );
    }

    pub(crate) fn apply_obj_store_downloaded(
        &mut self,
        _connection_id: u64,
        result: ObjectStoreDownloadResult,
    ) {
        self.toasts.push(
            ToastLevel::Success,
            format!("{} → {}", result.name, result.file_path),
        );
    }

    fn refresh_object_store_after_mutation(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        bucket: String,
    ) {
        self.toasts
            .push(ToastLevel::Success, format!("{operation} succeeded"));
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

fn upsert_obj_store_bucket(
    buckets: &mut Vec<ObjectStoreBucketInfo>,
    bucket: ObjectStoreBucketInfo,
) {
    if let Some(existing) = buckets.iter_mut().find(|info| info.bucket == bucket.bucket) {
        *existing = bucket;
    } else {
        buckets.push(bucket);
        buckets.sort_by(|a, b| a.bucket.cmp(&b.bucket));
    }
}
