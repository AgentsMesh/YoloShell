use super::*;
use crate::process::FreezePoll;

const MISSING_FORCE_STATE_DETAIL: &str = "retained process-tree transition state is missing";
const MISSING_CLEANUP_DETAIL: &str = "retained process-tree cleanup authority is missing";

impl ResilienceProcessTree {
    pub fn begin_force(self) -> ResilienceFreezingProcessTree {
        let cleanup = RetainedProcessCleanup::new(self.0.resilience_retained_targets());
        ResilienceFreezingProcessTree::new(self.0.begin_force(), cleanup)
    }
}

impl ResilienceFreezingProcessTree {
    pub(super) fn new(inner: FreezingProcessTree, cleanup: RetainedProcessCleanup) -> Self {
        Self {
            inner: Some(inner),
            cleanup: Some(cleanup),
        }
    }

    pub fn root(&self) -> ProcessIdentity {
        self.inner().root()
    }

    pub fn identities(&self) -> Vec<ProcessIdentity> {
        durable_targets(self.inner().resilience_retained_targets())
    }

    pub fn kill_cleanup_is_armed(&self) -> bool {
        self.cleanup
            .as_ref()
            .is_some_and(RetainedProcessCleanup::is_committed_to_kill)
    }

    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    pub fn authority_snapshot(&self) -> String {
        self.inner().adversarial_authority_snapshot()
    }

    #[cfg(test)]
    pub(crate) fn state_snapshot_for_test(&self) -> String {
        format!("{:?}", self.inner().state_snapshot_for_test())
    }

    pub fn poll(mut self) -> Result<ResilienceFreezePoll, ResilienceTransitionError<Self>> {
        let ready = match self.inner_mut().prepare_poll() {
            Ok(ready) => ready,
            Err(source) => return Err(ResilienceTransitionError::new(self, source)),
        };
        match self.take_inner().commit_poll(ready) {
            FreezePoll::Pending(inner) => {
                let mut cleanup = self.take_cleanup();
                cleanup.absorb(inner.resilience_retained_targets());
                Ok(ResilienceFreezePoll::Pending(Self::new(inner, cleanup)))
            }
            FreezePoll::Frozen(inner) => {
                let mut cleanup = self.take_cleanup();
                cleanup.absorb(inner.resilience_retained_targets());
                Ok(ResilienceFreezePoll::Frozen(
                    ResilienceFrozenProcessTree::new(inner, cleanup),
                ))
            }
        }
    }

    fn inner(&self) -> &FreezingProcessTree {
        self.inner.as_ref().expect(MISSING_FORCE_STATE_DETAIL)
    }

    fn inner_mut(&mut self) -> &mut FreezingProcessTree {
        self.inner.as_mut().expect(MISSING_FORCE_STATE_DETAIL)
    }

    fn take_inner(&mut self) -> FreezingProcessTree {
        self.inner.take().expect(MISSING_FORCE_STATE_DETAIL)
    }

    fn take_cleanup(&mut self) -> RetainedProcessCleanup {
        self.cleanup.take().expect(MISSING_CLEANUP_DETAIL)
    }
}
