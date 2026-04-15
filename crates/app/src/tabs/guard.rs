use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;

use tokio_util::sync::CancellationToken;

/// Monotonic counter for backend correlation IDs (never recycled).
static NEXT_BACKEND_ID: AtomicU64 = AtomicU64::new(1);

/// Monotonic counter for load generation IDs (KV batch deduplication).
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Returns a globally unique, monotonically increasing backend ID.
pub fn next_backend_id() -> u64 {
    NEXT_BACKEND_ID.fetch_add(1, Ordering::Relaxed)
}

/// Returns a globally unique, monotonically increasing generation ID.
pub fn next_generation() -> u64 {
    NEXT_GENERATION.fetch_add(1, Ordering::Relaxed)
}

/// RAII guard for tab lifecycle. When dropped:
/// - Cancels the `CancellationToken`, signalling backend tasks to stop.
/// - Returns the display ID through the mpsc channel for recycling.
pub struct TabGuard {
    cancel: CancellationToken,
    display_id: Option<u32>,
    id_return: Option<Sender<u32>>,
}

impl TabGuard {
    /// Create a guard with a recycled display ID (for Publisher/Subscriber tabs).
    pub fn new(cancel: CancellationToken, display_id: u32, id_return: Sender<u32>) -> Self {
        Self {
            cancel,
            display_id: Some(display_id),
            id_return: Some(id_return),
        }
    }

    /// Create a guard without a display ID (for resource tabs like Stream, KvBucket, etc.).
    pub fn new_without_id(cancel: CancellationToken) -> Self {
        Self {
            cancel,
            display_id: None,
            id_return: None,
        }
    }

    pub fn display_id(&self) -> Option<u32> {
        self.display_id
    }

    /// Create a `TaskCancellation` for passing to backend commands.
    pub fn cancellation(&self) -> nats_backend::TaskCancellation {
        nats_backend::TaskCancellation::new(self.cancel.child_token())
    }
}

impl Drop for TabGuard {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let (Some(id), Some(sender)) = (self.display_id, self.id_return.take()) {
            let _ = sender.send(id);
        }
    }
}

impl std::fmt::Debug for TabGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabGuard")
            .field("display_id", &self.display_id)
            .field("is_cancelled", &self.cancel.is_cancelled())
            .finish()
    }
}
