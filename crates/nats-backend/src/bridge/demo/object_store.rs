use std::path::PathBuf;

use crate::event::BackendEvent;
use crate::models::{
    ObjectStoreBucketConfigInput, ObjectStoreBucketInfo, ObjectStoreDownloadResult,
    ObjectStoreObjectInfo,
};

use super::DemoState;
use super::fixtures::{self, DEMO_TIME};

impl DemoState {
    pub(super) fn list_object_store_buckets(&mut self, connection_id: u64) {
        self.push(BackendEvent::ObjectStoreBucketsListed {
            connection_id,
            buckets: self.object_buckets.values().cloned().collect(),
        });
    }

    pub(super) fn create_object_store_bucket(
        &mut self,
        connection_id: u64,
        config: ObjectStoreBucketConfigInput,
    ) {
        let bucket = ObjectStoreBucketInfo {
            bucket: config.bucket.clone(),
            description: config.description.unwrap_or_default(),
            storage: fixtures::storage_label(config.storage),
            bytes: 0,
            max_bytes: config.max_bytes.unwrap_or(-1),
            object_count: 0,
            num_replicas: config.num_replicas.unwrap_or(1),
        };
        self.object_buckets
            .insert(config.bucket.clone(), bucket.clone());
        self.objects.entry(config.bucket).or_default();
        self.push(BackendEvent::ObjectStoreBucketCreated {
            connection_id,
            bucket,
        });
    }

    pub(super) fn delete_object_store_bucket(&mut self, connection_id: u64, bucket: String) {
        self.object_buckets.remove(&bucket);
        self.objects.remove(&bucket);
        self.push(BackendEvent::ObjectStoreBucketDeleted {
            connection_id,
            bucket,
        });
    }

    pub(super) fn list_objects(&mut self, connection_id: u64, bucket: String) {
        let objects = self
            .objects
            .get(&bucket)
            .into_iter()
            .flat_map(|items| items.values().cloned())
            .collect();
        self.push(BackendEvent::ObjectStoreObjectsListed {
            connection_id,
            bucket,
            objects,
        });
    }

    pub(super) fn upload_object(
        &mut self,
        connection_id: u64,
        bucket: String,
        name: String,
        data: Vec<u8>,
    ) {
        let object = ObjectStoreObjectInfo {
            bucket: bucket.clone(),
            name: name.clone(),
            description: String::new(),
            size: data.len(),
            chunks: usize::from(!data.is_empty()),
            modified: Some(DEMO_TIME.into()),
            digest: Some(format!("demo-{:#x}", data.len())),
        };
        self.objects
            .entry(bucket.clone())
            .or_default()
            .insert(name, object.clone());
        self.sync_object_bucket(&bucket);
        self.push(BackendEvent::ObjectStoreObjectUploaded {
            connection_id,
            object,
        });
    }

    pub(super) fn download_object(
        &mut self,
        connection_id: u64,
        bucket: String,
        name: String,
        file_path: PathBuf,
    ) {
        let size = self
            .objects
            .get(&bucket)
            .and_then(|items| items.get(&name))
            .map_or(0, |object| object.size as u64);
        self.push(BackendEvent::ObjectStoreObjectDownloaded {
            connection_id,
            result: ObjectStoreDownloadResult {
                bucket,
                name,
                file_path: file_path.display().to_string(),
                size,
            },
        });
    }

    pub(super) fn delete_object(&mut self, connection_id: u64, bucket: String, name: String) {
        if let Some(objects) = self.objects.get_mut(&bucket) {
            objects.remove(&name);
        }
        self.sync_object_bucket(&bucket);
        self.push(BackendEvent::ObjectStoreObjectDeleted {
            connection_id,
            bucket,
            name,
        });
    }

    fn sync_object_bucket(&mut self, bucket: &str) {
        let Some(info) = self.object_buckets.get_mut(bucket) else {
            return;
        };
        let objects = self.objects.get(bucket);
        info.object_count = objects.map_or(0, |items| items.len() as u64);
        info.bytes = objects.map_or(0, |items| {
            items.values().map(|object| object.size as u64).sum()
        });
    }
}
