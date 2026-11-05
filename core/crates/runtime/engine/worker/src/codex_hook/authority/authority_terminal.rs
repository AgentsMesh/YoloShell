use super::LifecycleAuthority;
use crate::agent::hooks::{
    HookViolation, NormalizedLifecycle, ProcessExitDisposition, ProcessIdentity,
};
use anyhow::Result;

pub(in crate::worker::codex_hook) enum CommittedProcessExit {
    Accepted(ProcessExitDisposition, NormalizedLifecycle),
    Rejected(HookViolation, NormalizedLifecycle),
}

impl LifecycleAuthority {
    pub(in crate::worker::codex_hook) fn process_exit_committed(
        &mut self,
        observed: ProcessIdentity,
    ) -> Result<CommittedProcessExit> {
        let (installed_revision, mut candidate) = self.candidate()?;
        let result = candidate.process_exit_with_identity(observed);
        let committed_state = self.install_candidate(installed_revision, candidate)?;
        match result {
            Ok(disposition) => Ok(CommittedProcessExit::Accepted(disposition, committed_state)),
            Err(violation) => Ok(CommittedProcessExit::Rejected(violation, committed_state)),
        }
    }
}
