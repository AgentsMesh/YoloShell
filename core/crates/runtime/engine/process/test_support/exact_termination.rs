use super::{RetainedExactAuthority, RetainedExactProcess};

impl RetainedExactProcess {
    pub fn fail_next_refresh_for_test(&mut self, detail: &str) {
        self.tree_mut().fail_next_refresh(detail);
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn is_exact_observer_for_test(&self) -> bool {
        matches!(&self.authority, RetainedExactAuthority::ExactObserver(_))
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn state_snapshot_for_test(&self) -> crate::CapturedProcessTreeStateSnapshot {
        self.tree().state_snapshot_for_test()
    }
}
