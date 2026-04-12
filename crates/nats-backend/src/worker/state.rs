use std::collections::HashMap;

use async_nats::Client;
use tokio::task::JoinHandle;

#[derive(Default)]
pub(crate) struct WorkerState {
    pub(crate) clients: HashMap<u64, Client>,
    pub(crate) subscriptions: HashMap<(u64, String), JoinHandle<()>>,
}
