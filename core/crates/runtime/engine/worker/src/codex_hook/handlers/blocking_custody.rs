use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;

#[derive(Clone, Default)]
pub(super) struct BlockingSettlementCustody {
    inner: Arc<SettlementState>,
}

pub(super) struct BlockingSettlementLease {
    inner: Arc<SettlementState>,
}

#[derive(Default)]
struct SettlementState {
    active: AtomicUsize,
    changed: Notify,
}

impl BlockingSettlementCustody {
    pub(super) fn admit(&self) -> BlockingSettlementLease {
        self.inner.active.fetch_add(1, Ordering::AcqRel);
        BlockingSettlementLease {
            inner: Arc::clone(&self.inner),
        }
    }

    pub(super) async fn wait_idle(&self) {
        loop {
            let changed = self.inner.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.inner.active.load(Ordering::Acquire) == 0 {
                return;
            }
            changed.await;
        }
    }
}

impl Drop for BlockingSettlementLease {
    fn drop(&mut self) {
        if self.inner.active.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.inner.changed.notify_waiters();
        }
    }
}
