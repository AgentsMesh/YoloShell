use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

#[derive(Clone, Default)]
pub(in super::super) struct ProbeBarrier {
    state: Arc<ProbeBarrierState>,
}

#[derive(Default)]
struct ProbeBarrierState {
    reached: AtomicBool,
    released: AtomicBool,
    reached_changed: Notify,
    released_changed: Notify,
}

impl ProbeBarrier {
    pub(in super::super) async fn pause_async(&self, cancelled: &AtomicBool) {
        self.state.reached.store(true, Ordering::Release);
        self.state.reached_changed.notify_waiters();
        loop {
            let changed = self.state.released_changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.state.released.load(Ordering::Acquire) || cancelled.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }

    pub(in super::super) async fn wait_until_reached(&self) {
        loop {
            let changed = self.state.reached_changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.state.reached.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }

    pub(in super::super) fn release(&self) {
        self.state.released.store(true, Ordering::Release);
        self.state.released_changed.notify_waiters();
    }
}
