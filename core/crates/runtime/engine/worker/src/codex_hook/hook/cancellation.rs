use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

const GENERATION_FENCE_INCREMENT: usize = 1;
const GENERATION_FENCE_UNDERFLOW: &str = "Codex ingress generation fence count underflowed";

#[derive(Clone, Default)]
pub(super) struct IngressCancellation {
    inner: Arc<CancellationState>,
}

#[derive(Default)]
struct CancellationState {
    status: Mutex<CancellationStatus>,
    changed: Notify,
}

#[derive(Default)]
struct CancellationStatus {
    requested: bool,
    active_fences: usize,
}

pub(super) struct IngressGenerationFence {
    inner: Arc<CancellationState>,
}

impl IngressCancellation {
    pub(super) fn cancel(&self) {
        let mut status = self
            .inner
            .status
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !status.requested {
            status.requested = true;
            drop(status);
            self.inner.changed.notify_waiters();
        }
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.inner
            .status
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .requested
    }

    // Fence acquisition is the linearization point for a commit or response. A fence acquired
    // before cancellation may finish; cancellation prevents every later acquisition.
    pub(super) fn try_fence(&self) -> Option<IngressGenerationFence> {
        let mut status = self
            .inner
            .status
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if status.requested {
            return None;
        }
        status.active_fences = status
            .active_fences
            .checked_add(GENERATION_FENCE_INCREMENT)?;
        Some(IngressGenerationFence {
            inner: Arc::clone(&self.inner),
        })
    }

    pub(super) async fn cancelled(&self) {
        loop {
            let notified = self.inner.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

impl Drop for IngressGenerationFence {
    fn drop(&mut self) {
        let mut status = self
            .inner
            .status
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        status.active_fences = status
            .active_fences
            .checked_sub(GENERATION_FENCE_INCREMENT)
            .expect(GENERATION_FENCE_UNDERFLOW);
    }
}
