use tokio_util::sync::CancellationToken;

/// Newtype wrapper around `CancellationToken` that implements `Debug`.
///
/// `CancellationToken` from tokio-util does not implement `Debug`, but
/// `BackendCommand` derives it. This wrapper provides a manual `Debug` impl.
#[derive(Clone)]
pub struct TaskCancellation(CancellationToken);

impl std::fmt::Debug for TaskCancellation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskCancellation")
            .field("is_cancelled", &self.0.is_cancelled())
            .finish()
    }
}

impl TaskCancellation {
    pub fn new(token: CancellationToken) -> Self {
        Self(token)
    }

    pub fn token(&self) -> &CancellationToken {
        &self.0
    }

    pub fn into_token(self) -> CancellationToken {
        self.0
    }

    pub fn child_token(&self) -> CancellationToken {
        self.0.child_token()
    }
}
