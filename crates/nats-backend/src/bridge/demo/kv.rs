use crate::event::{BackendEvent, BackendOperation};
use crate::models::{KvBucketConfigInput, KvBucketInfo, KvEntryInfo, KvHistoryItem, KvKeyBatch};

use super::DemoState;
use super::fixtures::{self, DEMO_TIME};

impl DemoState {
    pub(super) fn list_kv_buckets(&mut self, connection_id: u64) {
        self.push(BackendEvent::KvBucketsListed {
            connection_id,
            buckets: self.kv_buckets.values().cloned().collect(),
        });
    }

    pub(super) fn upsert_kv_bucket(&mut self, connection_id: u64, config: KvBucketConfigInput) {
        let created = !self.kv_buckets.contains_key(&config.bucket);
        let bucket = KvBucketInfo {
            bucket: config.bucket.clone(),
            stored_history_values: 0,
            history_depth: config.history,
            max_age_secs: config.max_age.map_or(0, |age| age.as_secs()),
            max_age_nanos: config.max_age.map_or(0, |age| age.subsec_nanos() as u64),
            description: config.description.unwrap_or_default(),
            storage: fixtures::storage_label(config.storage),
            bytes: 0,
            max_bytes: config.max_bytes.unwrap_or(-1),
            max_value_size: i64::from(config.max_value_size.unwrap_or(-1)),
            num_replicas: config.num_replicas.unwrap_or(1),
        };
        self.kv_buckets.insert(config.bucket, bucket.clone());
        self.push(if created {
            BackendEvent::KvBucketCreated {
                connection_id,
                bucket,
            }
        } else {
            BackendEvent::KvBucketUpdated {
                connection_id,
                bucket,
            }
        });
    }

    pub(super) fn delete_kv_bucket(&mut self, connection_id: u64, bucket: String) {
        self.kv_buckets.remove(&bucket);
        self.kv_entries.remove(&bucket);
        self.push(BackendEvent::KvBucketDeleted {
            connection_id,
            bucket,
        });
    }

    pub(super) fn list_kv_keys(&mut self, connection_id: u64, bucket: String, generation: u64) {
        let keys = self
            .kv_entries
            .get(&bucket)
            .map(|items| items.keys().cloned().collect())
            .unwrap_or_default();
        self.push(BackendEvent::KvKeysListed {
            connection_id,
            batch: KvKeyBatch {
                bucket,
                keys,
                done: true,
                generation,
            },
        });
    }

    pub(super) fn get_kv_entry(&mut self, connection_id: u64, bucket: String, key: String) {
        let item = self
            .kv_entries
            .get(&bucket)
            .and_then(|items| items.get(&key))
            .and_then(|history| history.last());
        self.push(BackendEvent::KvEntryFetched {
            connection_id,
            entry: KvEntryInfo {
                bucket,
                key,
                value: item.map_or_else(Vec::new, |item| item.value.clone()),
                revision: item.map(|item| item.revision),
                delta: item.map(|item| item.delta),
                created: item.map(|item| item.created.clone()),
                operation: item.map(|item| item.operation.clone()),
            },
        });
    }

    pub(super) fn put_kv_entry(
        &mut self,
        connection_id: u64,
        bucket: String,
        key: String,
        value: Vec<u8>,
    ) {
        self.mutate_kv(
            connection_id,
            BackendOperation::PutKvEntry,
            bucket,
            key,
            value,
            "Put",
        );
    }

    pub(super) fn delete_kv_entry(&mut self, connection_id: u64, bucket: String, key: String) {
        self.mutate_kv(
            connection_id,
            BackendOperation::DeleteKvEntry,
            bucket,
            key,
            Vec::new(),
            "Delete",
        );
    }

    pub(super) fn purge_kv_entry(&mut self, connection_id: u64, bucket: String, key: String) {
        if let Some(entries) = self.kv_entries.get_mut(&bucket) {
            entries.remove(&key);
        }
        self.push(BackendEvent::KvEntryMutated {
            connection_id,
            operation: BackendOperation::PurgeKvEntry,
            bucket,
            key,
        });
    }

    pub(super) fn get_kv_history(&mut self, connection_id: u64, bucket: String, key: String) {
        let history = self
            .kv_entries
            .get(&bucket)
            .and_then(|items| items.get(&key))
            .cloned()
            .unwrap_or_default();
        self.push(BackendEvent::KvHistoryFetched {
            connection_id,
            bucket,
            key,
            history,
        });
    }

    fn mutate_kv(
        &mut self,
        connection_id: u64,
        operation: BackendOperation,
        bucket: String,
        key: String,
        value: Vec<u8>,
        operation_name: &str,
    ) {
        self.revision += 1;
        let history = self
            .kv_entries
            .entry(bucket.clone())
            .or_default()
            .entry(key.clone())
            .or_default();
        history.push(KvHistoryItem {
            key: key.clone(),
            value,
            revision: self.revision,
            delta: 0,
            created: DEMO_TIME.into(),
            operation: operation_name.into(),
        });
        if history.len() > 10 {
            history.remove(0);
        }
        self.push(BackendEvent::KvEntryMutated {
            connection_id,
            operation,
            bucket,
            key,
        });
    }
}
