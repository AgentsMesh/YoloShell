use super::ReaderLifecycle;
use std::sync::atomic::{AtomicBool, Ordering};

impl ReaderLifecycle {
    pub(crate) fn publish_child_waited(&self, waited: &AtomicBool) {
        let _progress = self.lock_progress();
        #[cfg(test)]
        self.child_wait_publication_after_lock.pause_if_armed();
        waited.store(true, Ordering::Release);
        self.changed.notify_all();
    }
}
