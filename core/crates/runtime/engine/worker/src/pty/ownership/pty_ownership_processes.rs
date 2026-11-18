use super::{OWNERSHIP_TREE_MISSING_DETAIL, PtyOwnership};
use crate::process::{CapturedProcessTree, DirectChildExitStatus, UnreapedRootExit};
#[cfg(test)]
use crate::worker::pty_force_termination::PtyForceTermination;
use crate::worker::pty_force_termination::{ForceOutput, ForceTerminationPoll};
use crate::worker::pty_process_custody::PtyProcessCustody;
use anyhow::{Context, Result};

impl PtyOwnership {
    pub(crate) fn require_destructive_session(
        &self,
    ) -> std::result::Result<(), crate::process::DestructiveSessionUnavailable> {
        match self.custody().processes.as_ref() {
            Some(PtyProcessCustody::Tracking(tree)) => tree.require_destructive_session(),
            _ => Err(crate::process::DestructiveSessionUnavailable::new()),
        }
    }

    pub(crate) fn tree_mut(&mut self) -> &mut CapturedProcessTree {
        self.custody_mut()
            .processes
            .as_mut()
            .expect(OWNERSHIP_TREE_MISSING_DETAIL)
            .tracking_mut()
    }

    pub(crate) fn process_custody_mut(&mut self) -> &mut Option<PtyProcessCustody> {
        &mut self.custody_mut().processes
    }

    pub(crate) fn poll_force(
        &mut self,
        output: &mut ForceOutput<'_>,
    ) -> Result<ForceTerminationPoll> {
        let custody = self.custody_mut();
        custody.force.poll(&mut custody.processes, output)
    }

    pub(crate) fn record_force_request(&mut self, force: bool) {
        self.custody_mut().force.record_force_request(force);
    }

    #[cfg(test)]
    pub(crate) fn force_mut_for_test(&mut self) -> &mut PtyForceTermination {
        &mut self.custody_mut().force
    }

    pub(crate) fn tracking_tree_mut(&mut self) -> Option<&mut CapturedProcessTree> {
        self.custody_mut()
            .processes
            .as_mut()
            .and_then(PtyProcessCustody::try_tracking_mut)
    }

    pub(crate) fn session_is_retired(&self) -> bool {
        self.custody()
            .processes
            .as_ref()
            .is_some_and(PtyProcessCustody::session_is_retired)
    }

    pub(crate) fn mark_root_exited_unreaped(&mut self, proof: UnreapedRootExit) -> Result<()> {
        self.custody_mut()
            .processes
            .as_mut()
            .expect(OWNERSHIP_TREE_MISSING_DETAIL)
            .mark_root_exited_unreaped(proof)?;
        self.custody_mut().root_exit_proof = Some(proof);
        Ok(())
    }

    pub(crate) fn wait_root_exited(&mut self) -> Result<DirectChildExitStatus> {
        let proof = self
            .custody()
            .root_exit_proof
            .context(super::OWNERSHIP_AUTHORITY_MISSING_DETAIL)?;
        self.custody_mut()
            .authority
            .as_mut()
            .context(super::OWNERSHIP_AUTHORITY_MISSING_DETAIL)?
            .wait_exited(proof)
    }
}
