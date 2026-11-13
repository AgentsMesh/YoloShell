use super::CapturedProcessTree;
use super::ownership::SessionOwnership;
use super::root_exit::RootExitState;
use crate::process::{ExplicitBootstrapExecFailureProof, PtyMasterAliasesClosed};
use anyhow::{Context, Result, ensure};

const PTY_PROOF_UNAVAILABLE_CONTEXT: &str =
    "explicit bootstrap exec failure has no bound PTY proof";
const FAILURE_ALREADY_RECORDED_DETAIL: &str =
    "explicit bootstrap exec failure was already recorded";
const FAILURE_NOT_RECORDED_DETAIL: &str = "explicit bootstrap exec failure was not recorded";
const ROOT_EXIT_REQUIRED_DETAIL: &str =
    "explicit bootstrap exec failure requires an exact unreaped root exit";
const TRACKING_REQUIRED_DETAIL: &str =
    "explicit bootstrap exec failure requires a tracking process tree";

pub struct ExplicitBootstrapFailureRetirementReady<'tree> {
    tree: &'tree mut CapturedProcessTree,
}

impl CapturedProcessTree {
    pub fn record_explicit_bootstrap_exec_failure(
        &mut self,
        failure: ExplicitBootstrapExecFailureProof,
    ) -> Result<()> {
        ensure!(
            self.ownership == SessionOwnership::Tracking,
            TRACKING_REQUIRED_DETAIL
        );
        ensure!(
            !self.explicit_bootstrap_failure,
            FAILURE_ALREADY_RECORDED_DETAIL
        );
        self.pty_proof
            .as_ref()
            .context(PTY_PROOF_UNAVAILABLE_CONTEXT)?
            .accept_explicit_bootstrap_exec_failure(failure)?;
        self.explicit_bootstrap_failure = true;
        Ok(())
    }

    pub fn explicit_bootstrap_exec_failure_is_pending(&self) -> bool {
        self.explicit_bootstrap_failure && self.ownership != SessionOwnership::Retired
    }

    pub fn prepare_explicit_bootstrap_failure_retirement(
        &mut self,
        aliases: PtyMasterAliasesClosed,
    ) -> Result<ExplicitBootstrapFailureRetirementReady<'_>> {
        ensure!(self.explicit_bootstrap_failure, FAILURE_NOT_RECORDED_DETAIL);
        ensure!(
            self.ownership == SessionOwnership::Tracking,
            TRACKING_REQUIRED_DETAIL
        );
        ensure!(
            self.root_exit == RootExitState::ExitedUnreaped,
            ROOT_EXIT_REQUIRED_DETAIL
        );
        self.mark_pty_master_released(aliases)?;
        Ok(ExplicitBootstrapFailureRetirementReady { tree: self })
    }
}

impl ExplicitBootstrapFailureRetirementReady<'_> {
    pub fn commit(self) {
        self.tree.explicit_bootstrap_failure = false;
        self.tree.retire_session();
    }
}
