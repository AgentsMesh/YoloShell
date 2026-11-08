use super::Journal;

impl Journal {
    pub fn set_last_sequence_for_test(&mut self, sequence: u64) {
        self.last_sequence = sequence;
    }

    pub fn inject_reconcile_failure(&mut self) {
        self.inject_reconcile_failures(1);
    }

    pub fn inject_reconcile_failures(&mut self, count: usize) {
        self.reconcile_faults = count;
    }

    pub fn inject_retained_reconcile_failures(&mut self, count: usize) {
        self.retained_reconcile_faults = count;
    }

    pub fn inject_retained_post_install_failure(&mut self) {
        super::tests_retained_hooks::inject_post_install_failure();
    }

    pub fn inject_retained_parent_sync_failure(&mut self) {
        super::tests_retained_hooks::inject_parent_sync_failure();
    }

    pub fn inject_retained_confirmation_failure(&mut self) {
        super::tests_retained_hooks::inject_confirm_failure();
    }
}
