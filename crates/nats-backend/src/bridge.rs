#[cfg(feature = "native")]
use tokio::sync::mpsc;

#[cfg(feature = "native")]
use crate::command::BackendCommand;
#[cfg(feature = "native")]
use crate::event::BackendEvent;

/// Handle to communicate with the async backend from the UI thread.
#[cfg(feature = "native")]
pub struct BackendHandle {
    cmd_tx: mpsc::UnboundedSender<BackendCommand>,
    evt_rx: mpsc::Receiver<BackendEvent>,
}

#[cfg(feature = "native")]
impl BackendHandle {
    /// Spawn the Tokio runtime on a background thread and return a handle.
    ///
    /// Panics if the Tokio runtime cannot be created (propagated from the
    /// background thread via a oneshot channel so the failure is visible on
    /// the calling thread).
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<BackendCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<BackendEvent>(4096);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => {
                    let _ = ready_tx.send(Ok(()));
                    rt
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            };
            rt.block_on(async move {
                crate::worker::run_worker(cmd_rx, evt_tx).await;
            });
        });

        ready_rx
            .recv()
            .expect("backend thread terminated before reporting status")
            .expect("failed to create Tokio runtime");

        Self { cmd_tx, evt_rx }
    }

    /// Send a command to the backend worker.
    pub fn send(&self, cmd: BackendCommand) {
        if let Err(e) = self.cmd_tx.send(cmd) {
            tracing::error!("Failed to send command to backend: {e}");
        }
    }

    /// Try to receive the next event from the backend (non-blocking).
    pub fn try_recv(&mut self) -> Option<BackendEvent> {
        self.evt_rx.try_recv().ok()
    }

    /// Drain all pending events from the backend.
    pub fn drain_events(&mut self) -> Vec<BackendEvent> {
        let mut events = Vec::new();
        while let Some(evt) = self.try_recv() {
            events.push(evt);
        }
        events
    }

    /// Return the delay before the UI should poll this backend again.
    ///
    /// The native backend wakes the UI through its normal event flow and does
    /// not require a scheduled poll.
    pub fn next_wakeup(&self) -> Option<std::time::Duration> {
        None
    }
}

#[cfg(all(feature = "demo", not(feature = "native")))]
mod demo;
#[cfg(all(feature = "demo", not(feature = "native")))]
pub use demo::BackendHandle;
