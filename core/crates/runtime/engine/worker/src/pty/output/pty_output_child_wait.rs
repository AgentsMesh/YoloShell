use super::PtyOutputRecovery;
use std::sync::atomic::Ordering;

impl PtyOutputRecovery {
    pub(crate) fn publish_direct_child_waited(&self) {
        match (self.child_waited.as_ref(), self.reader_lifecycle.as_ref()) {
            (Some(waited), Some(lifecycle)) => lifecycle.publish_child_waited(waited),
            (Some(waited), None) => waited.store(true, Ordering::Release),
            (None, _) => {}
        }
    }
}
