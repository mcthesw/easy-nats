use std::collections::HashMap;

use async_nats::Client;
use tokio::task::JoinHandle;

#[derive(Default)]
pub(crate) struct WorkerState {
    pub(crate) clients: HashMap<u64, Client>,
    pub(crate) subscriptions: HashMap<(u64, u64, String), JoinHandle<()>>,
    /// Tracks spawned KV list tasks: (connection_id, bucket) → (generation, handle)
    pub(crate) kv_tasks: HashMap<(u64, String), (u64, JoinHandle<()>)>,
}
