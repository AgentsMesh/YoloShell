use crate::process::{ProcessIdentity, RootWaitAuthority, UnreapedRootExit};
use anyhow::{Result, ensure};

pub(crate) const ROOT_AUTHORITY_MISMATCH_DETAIL: &str =
    "PTY root observer has different child authority";

pub(crate) trait RootExitObserver: Send {
    fn poll_exited_unreaped(
        &mut self,
        authority: &RootWaitAuthority,
    ) -> Result<Option<UnreapedRootExit>>;
}

pub(crate) struct WaitidRootExitObserver {
    pub(crate) root: ProcessIdentity,
}

impl WaitidRootExitObserver {
    pub(crate) fn new(root: ProcessIdentity) -> Self {
        Self { root }
    }
}

impl RootExitObserver for WaitidRootExitObserver {
    fn poll_exited_unreaped(
        &mut self,
        authority: &RootWaitAuthority,
    ) -> Result<Option<UnreapedRootExit>> {
        ensure!(
            self.root == authority.root(),
            ROOT_AUTHORITY_MISMATCH_DETAIL
        );
        authority.poll_exited_unreaped()
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_proof_tests.rs"]
mod owner_proof_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/pty_exit_observer_tests.rs"]
mod tests;
