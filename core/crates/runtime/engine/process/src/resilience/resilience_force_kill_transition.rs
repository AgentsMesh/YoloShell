use super::*;
use crate::process::{KillCommitPoll, KillCommitReady, KillPoll};

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/resilience/resilience_force_kill_transition_test_hook.rs"]
mod test_hook;
#[cfg(all(test, feature = "yolo-integration-tests"))]
pub(crate) use test_hook::install_test_kill_commit_handoff_panic;

const MISSING_FORCE_STATE_DETAIL: &str = "retained process-tree transition state is missing";
const MISSING_CLEANUP_DETAIL: &str = "retained process-tree cleanup authority is missing";

impl ResilienceFrozenProcessTree {
    pub(super) fn new(inner: FrozenProcessTree, cleanup: RetainedProcessCleanup) -> Self {
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

    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    pub fn authority_snapshot(&self) -> String {
        self.inner().adversarial_authority_snapshot()
    }

    pub fn arm_kill_cleanup(&mut self) {
        self.cleanup_mut().commit_to_kill();
    }

    pub fn kill_cleanup_is_armed(&self) -> bool {
        self.cleanup
            .as_ref()
            .is_some_and(RetainedProcessCleanup::is_committed_to_kill)
    }

    pub fn kill(self) -> Result<ResilienceKillCommitPoll, ResilienceTransitionError<Self>> {
        self.kill_with_fence(|| Ok(()))
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn kill_with_destructive_commit_fence(
        self,
        commit_fence: impl FnMut() -> anyhow::Result<()>,
    ) -> Result<ResilienceKillCommitPoll, ResilienceTransitionError<Self>> {
        self.kill_with_fence(commit_fence)
    }

    #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
    pub(crate) fn downgrade_kill_basis_to_ordinary_for_test(&mut self) {
        self.inner
            .as_mut()
            .expect(MISSING_FORCE_STATE_DETAIL)
            .downgrade_kill_basis_to_ordinary_for_test();
    }

    fn kill_with_fence(
        mut self,
        commit_fence: impl FnMut() -> anyhow::Result<()>,
    ) -> Result<ResilienceKillCommitPoll, ResilienceTransitionError<Self>> {
        let ready = match self.prepare_kill(commit_fence) {
            Ok(ready) => ready,
            Err(source) => return Err(ResilienceTransitionError::new(self, source)),
        };
        let inner = self.take_inner();
        #[cfg(all(test, feature = "yolo-integration-tests"))]
        test_hook::panic_at_handoff();
        let transition = inner.commit_kill(ready);
        let mut cleanup = self.take_cleanup();
        Ok(match transition {
            KillCommitPoll::Refreeze(inner) => {
                cleanup.absorb(inner.resilience_retained_targets());
                ResilienceKillCommitPoll::Refreeze(ResilienceFreezingProcessTree::new(
                    inner, cleanup,
                ))
            }
            KillCommitPoll::Pending(inner) => {
                cleanup.absorb(inner.resilience_retained_targets());
                ResilienceKillCommitPoll::Pending(ResilienceKillPendingProcessTree::new(
                    inner, cleanup,
                ))
            }
        })
    }

    fn prepare_kill(
        &mut self,
        commit_fence: impl FnMut() -> anyhow::Result<()>,
    ) -> anyhow::Result<KillCommitReady> {
        let version = self.inner().closure_version();
        let inner = self.inner.as_mut().expect(MISSING_FORCE_STATE_DETAIL);
        let cleanup = self.cleanup.as_mut().expect(MISSING_CLEANUP_DETAIL);
        inner.prepare_kill_with_destructive_commit(version, commit_fence, || {
            cleanup.commit_to_kill()
        })
    }

    fn inner(&self) -> &FrozenProcessTree {
        self.inner.as_ref().expect(MISSING_FORCE_STATE_DETAIL)
    }

    fn take_inner(&mut self) -> FrozenProcessTree {
        self.inner.take().expect(MISSING_FORCE_STATE_DETAIL)
    }

    fn cleanup_mut(&mut self) -> &mut RetainedProcessCleanup {
        self.cleanup.as_mut().expect(MISSING_CLEANUP_DETAIL)
    }

    fn take_cleanup(&mut self) -> RetainedProcessCleanup {
        self.cleanup.take().expect(MISSING_CLEANUP_DETAIL)
    }
}

impl ResilienceKillPendingProcessTree {
    pub(super) fn new(inner: KillPendingProcessTree, cleanup: RetainedProcessCleanup) -> Self {
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

    pub fn poll(mut self) -> Result<ResilienceKillPoll, ResilienceTransitionError<Self>> {
        let ready = match self.inner_mut().prepare_poll() {
            Ok(ready) => ready,
            Err(source) => return Err(ResilienceTransitionError::new(self, source)),
        };
        match self.take_inner().commit_poll(ready) {
            KillPoll::Refreeze(inner) => {
                let mut cleanup = self.take_cleanup();
                cleanup.absorb(inner.resilience_retained_targets());
                Ok(ResilienceKillPoll::Refreeze(
                    ResilienceFreezingProcessTree::new(inner, cleanup),
                ))
            }
            KillPoll::Pending(inner) => {
                let mut cleanup = self.take_cleanup();
                cleanup.absorb(inner.resilience_retained_targets());
                Ok(ResilienceKillPoll::Pending(Self::new(inner, cleanup)))
            }
            KillPoll::Gone(inner) => {
                let mut cleanup = self.take_cleanup();
                cleanup.disarm();
                Ok(ResilienceKillPoll::Gone(ResilienceProducersGone { inner }))
            }
        }
    }

    fn inner(&self) -> &KillPendingProcessTree {
        self.inner.as_ref().expect(MISSING_FORCE_STATE_DETAIL)
    }

    fn inner_mut(&mut self) -> &mut KillPendingProcessTree {
        self.inner.as_mut().expect(MISSING_FORCE_STATE_DETAIL)
    }

    fn take_inner(&mut self) -> KillPendingProcessTree {
        self.inner.take().expect(MISSING_FORCE_STATE_DETAIL)
    }

    fn take_cleanup(&mut self) -> RetainedProcessCleanup {
        self.cleanup.take().expect(MISSING_CLEANUP_DETAIL)
    }
}
