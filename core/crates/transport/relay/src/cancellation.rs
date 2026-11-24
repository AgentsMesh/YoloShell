use super::RelayError;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

#[derive(Clone, Debug, Default)]
pub struct RelayCancellation {
    inner: Arc<CancellationState>,
}

#[derive(Debug, Default)]
struct CancellationState {
    cancelled: AtomicBool,
    changed: Notify,
}

impl RelayCancellation {
    pub fn cancel(&self) {
        if !self.inner.cancelled.swap(true, Ordering::SeqCst) {
            self.inner.changed.notify_waiters();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    pub(crate) fn check(&self) -> Result<(), RelayError> {
        if self.is_cancelled() {
            Err(RelayError::Cancelled)
        } else {
            Ok(())
        }
    }

    pub(crate) async fn cancelled(&self) {
        loop {
            let changed = self.inner.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            changed.await;
        }
    }

    pub(crate) async fn wait<F>(&self, future: F) -> Result<F::Output, RelayError>
    where
        F: Future,
    {
        self.check()?;
        tokio::select! {
            biased;
            _ = self.cancelled() => Err(RelayError::Cancelled),
            output = future => Ok(output),
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/cancellation_tests.rs"]
mod tests;
