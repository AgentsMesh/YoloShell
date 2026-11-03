use super::*;

impl WorkerState {
    pub(super) fn publish_direct_child_waited(&self) {
        self.reader_lifecycle.publish_child_waited(&self.child_waited);
    }
}
