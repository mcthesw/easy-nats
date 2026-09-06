pub(crate) mod common;
pub(crate) mod guard;
mod kv;
pub(crate) mod log_viewer;
mod message_schemas;
mod metrics;
mod metrics_clients;
mod object_store;
mod publisher;
mod search_workspace;
mod server_info;
pub(crate) mod settings;
mod stream;
mod stream_consumers;
mod subscriber;
mod subscriber_detail;
pub(crate) mod types;
mod viewer;

pub(crate) use common::{
    KV_VALUE_SEARCH_BATCH, NormalizedSearchQuery, payload_input_format_selector,
};
pub use guard::{TabGuard, next_backend_id, next_generation};
pub use kv::kv_bucket_ui;
pub use message_schemas::message_schemas_ui;
pub use metrics::metrics_ui;
pub(crate) use metrics_clients::clients_ui;
pub use object_store::obj_store_bucket_ui;
pub use publisher::publisher_ui;
pub(crate) use search_workspace::{
    SearchWorkspaceBuildStats, append_search_workspace_results, search_workspace_ui,
    source_summary_from_tab,
};
pub use server_info::server_info_ui;
pub use stream::stream_ui;
pub use subscriber::subscriber_ui;
pub use types::{
    AppTabViewer, ClientStatusState, KvBucketState, MessageSchemasState, MetricsState,
    ObjectStoreBucketState, PreviewFetchState, PublisherState, ReceivedMessage, ResponseData,
    SearchResultLocator, SearchSourceId, SearchSourceSummary, SearchWorkspaceCacheKey,
    SearchWorkspaceResult, SearchWorkspaceState, ServerInfoState, StreamState, SubscriberState,
    TabAction, TabKind,
};

#[cfg(target_arch = "wasm32")]
pub(crate) use types::{SearchField, SearchResultKey};
