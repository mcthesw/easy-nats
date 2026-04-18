pub mod bridge;
pub mod cancellation;
pub mod command;
pub mod config;
pub mod connection;
pub mod event;
pub mod paths;
pub mod worker;

pub use bridge::BackendHandle;
pub use cancellation::TaskCancellation;
pub use command::BackendCommand;
pub use config::AppConfig;
pub use connection::{AuthMethod, ConnectionConfig, ConnectionStatus};
pub use event::{BackendEvent, BackendOperation, ConnectionStatusKind, MessageData};
pub use paths::{MigrationOutcome, ProjectPaths, migrate_legacy_file, migrate_legacy_on_startup};
