use crate::{
    RequestScope,
    lease_deadline::{LeaseDeadline, SharedLeaseDeadline},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

/// One ordered input stream. Views can retire queued work independently, while
/// an attempted input failure fences all views before the next serial dispatch.
#[derive(Debug)]
pub struct InputStream {
    pub(crate) failed: Arc<AtomicBool>,
    pub(crate) lease: SharedLeaseDeadline,
}

impl InputStream {
    pub fn new(expires: Instant) -> Self {
        let lease = Arc::new(LeaseDeadline::default());
        lease.initialize(expires);
        Self {
            failed: Arc::default(),
            lease,
        }
    }

    pub fn view(&self) -> RequestScope {
        RequestScope::input_view(self)
    }

    pub fn is_current(&self) -> bool {
        !self.failed.load(Ordering::Acquire) && Instant::now() < self.lease.current(Instant::now())
    }

    /// Only a confirmed renewal of the owner's exact writer may extend this.
    pub fn renew_authority(&self, expires: Instant) -> bool {
        if self.failed.load(Ordering::Acquire) {
            return false;
        }
        if !self.lease.renew(expires, Instant::now()) {
            self.retire();
            return false;
        }
        !self.failed.load(Ordering::Acquire)
    }

    pub fn retire(&self) {
        self.failed.store(true, Ordering::Release);
    }
}

impl Drop for InputStream {
    fn drop(&mut self) {
        self.retire();
    }
}
