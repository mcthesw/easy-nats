use std::collections::HashMap;
use std::time::Duration;

use async_nats::Client;
use tokio::task::JoinHandle;

pub(crate) struct WorkerState {
    pub(crate) clients: HashMap<u64, Client>,
    pub(crate) subscriptions: HashMap<(u64, u64, String), JoinHandle<()>>,
    /// Tracks spawned KV list tasks: (connection_id, bucket) → (generation, handle)
    pub(crate) kv_tasks: HashMap<(u64, String), (u64, JoinHandle<()>)>,
    pub(crate) http_client: reqwest::Client,
}

impl Default for WorkerState {
    fn default() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent(format!("easy-nats/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("valid HTTP client");

        Self {
            clients: HashMap::new(),
            subscriptions: HashMap::new(),
            kv_tasks: HashMap::new(),
            http_client,
        }
    }
}
