#[cfg(test)]
use super::PtySessionProof;
use super::{CapturedProcessTree, ProcessIdentity};
use anyhow::Result;

#[path = "resilience_force.rs"]
mod force;
#[cfg(all(test, feature = "yolo-integration-tests"))]
pub(crate) use force::install_test_kill_commit_handoff_panic;
pub use force::{
    ResilienceFreezePoll, ResilienceFreezingProcessTree, ResilienceFrozenProcessTree,
    ResilienceKillCommitPoll, ResilienceKillPendingProcessTree, ResilienceKillPoll,
    ResilienceProducersGone, ResilienceTransitionError,
};

pub struct ResilienceProcessTree(CapturedProcessTree);

impl ResilienceProcessTree {
    pub fn capture(root: ProcessIdentity) -> Result<Self> {
        CapturedProcessTree::capture(root).map(Self)
    }

    #[cfg(target_os = "linux")]
    pub fn capture_linux(
        root: ProcessIdentity,
        record: super::linux_containment::LinuxContainmentRecord,
    ) -> Result<Self> {
        let (containment, proof) = super::linux_containment::resilience::reopen(root, record)?;
        let mut tree = CapturedProcessTree::capture_pty(root, proof)?;
        tree.install_linux_containment(containment)?;
        Ok(Self(tree))
    }

    #[cfg(target_os = "linux")]
    pub fn capture_linux_with_rollback(
        root: ProcessIdentity,
        record: super::linux_containment::LinuxContainmentRecord,
    ) -> Result<(
        Self,
        super::linux_containment::ResilienceContainmentRollback,
    )> {
        let (containment, proof) = super::linux_containment::resilience::reopen(root, record)?;
        let rollback =
            super::linux_containment::ResilienceContainmentRollback::capture(&containment)?;
        let mut tree = CapturedProcessTree::capture_pty(root, proof)?;
        tree.install_linux_containment(containment)?;
        Ok((Self(tree), rollback))
    }

    #[cfg(test)]
    pub(crate) fn capture_pty(root: ProcessIdentity, proof: PtySessionProof) -> Result<Self> {
        CapturedProcessTree::capture_pty(root, proof).map(Self)
    }

    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    pub fn capture_custody(
        root: ProcessIdentity,
        custody: super::PtyCustodyRecoveryAuthority,
    ) -> Result<Self> {
        CapturedProcessTree::capture_pty(root, custody.into_proof(root)?).map(Self)
    }

    pub fn identities(&self) -> Vec<ProcessIdentity> {
        self.0.identities()
    }

    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    pub fn authority_snapshot(&self) -> String {
        self.0.adversarial_authority_snapshot()
    }

    pub fn terminate(&mut self, signal: i32) -> Result<Vec<ProcessIdentity>> {
        self.0.reconcile_retained_root_exit()?;
        self.0.terminate(signal)
    }

    pub fn cleanup_discovery_is_settled(&self) -> bool {
        self.0.descendant_observation() == super::DescendantObservation::Settled
    }
}

pub fn send_signal(pid: u32, signal: i32) -> Result<()> {
    if let Some(identity) = ProcessIdentity::try_capture(pid)? {
        identity.signal(signal)?;
    }
    Ok(())
}

pub fn observe_process_tree(root: ProcessIdentity) -> Result<Vec<ProcessIdentity>> {
    Ok(CapturedProcessTree::capture(root)?.identities())
}

pub fn terminate_process_tree(pid: u32, force: bool) -> Result<()> {
    let Some(root) = ProcessIdentity::try_capture(pid)? else {
        return Ok(());
    };
    let mut tree = match CapturedProcessTree::capture(root) {
        Ok(tree) => tree,
        Err(_) if !root.matches() => return Ok(()),
        Err(error) => return Err(error),
    };
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    tree.terminate(signal)?;
    Ok(())
}
