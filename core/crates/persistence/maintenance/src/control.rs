use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub(super) struct StopControl {
    requested: AtomicBool,
    mutex: Mutex<()>,
    wake: Condvar,
}

impl StopControl {
    pub(super) fn new() -> Self {
        Self {
            requested: AtomicBool::new(false),
            mutex: Mutex::new(()),
            wake: Condvar::new(),
        }
    }

    pub(super) fn requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }

    pub(super) fn request(&self) {
        self.requested.store(true, Ordering::Release);
        self.wake.notify_all();
    }

    pub(super) fn wait(&self, duration: Duration) -> bool {
        if self.requested() {
            return true;
        }
        let guard = self.mutex.lock().unwrap_or_else(|error| error.into_inner());
        let _ = self
            .wake
            .wait_timeout_while(guard, duration, |_| !self.requested())
            .unwrap_or_else(|error| error.into_inner());
        self.requested()
    }
}
