mod actions;
#[cfg(target_arch = "wasm32")]
mod demo;
mod editors;
mod events;
mod kv_results;
mod metrics_results;
mod model;
mod obj_store_results;
mod operation_results;
mod platform_actions;
mod pubsub_events;
mod search_workspace;
mod server_info_results;
mod sidebar;
mod stream_results;
mod ui;
mod util;
mod windows;

pub use model::EasyNatsApp;
