use tokio::sync::mpsc;

use crate::command::BackendCommand;
use crate::event::BackendEvent;

/// Handle to communicate with the async backend from the UI thread.
pub struct BackendHandle {
    cmd_tx: mpsc::UnboundedSender<BackendCommand>,
    evt_rx: mpsc::UnboundedReceiver<BackendEvent>,
}

impl BackendHandle {
    /// Spawn the Tokio runtime on a background thread and return a handle.
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<BackendCommand>();
        let (evt_tx, evt_rx) = mpsc::unbounded_channel::<BackendEvent>();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("failed to create Tokio runtime");
            rt.block_on(async move {
                crate::worker::run_worker(cmd_rx, evt_tx).await;
            });
        });

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
}
