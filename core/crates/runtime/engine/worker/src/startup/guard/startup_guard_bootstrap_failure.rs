use super::StartupChildGuard;
use crate::process::ExplicitBootstrapExecFailureProof;
use crate::worker::bootstrap_gate::ExplicitBootstrapExecFailure;
use anyhow::{Context, Result};

pub(crate) const STARTUP_TREE_UNAVAILABLE_CONTEXT: &str =
    "startup process tree is unavailable for explicit bootstrap exec failure";

impl StartupChildGuard {
    pub(crate) fn record_explicit_bootstrap_exec_failure(
        &mut self,
        failure: ExplicitBootstrapExecFailure,
    ) -> Result<()> {
        self.record_explicit_bootstrap_exec_failure_proof(failure.into_process_proof())
    }

    pub(crate) fn record_explicit_bootstrap_exec_failure_proof(
        &mut self,
        failure: ExplicitBootstrapExecFailureProof,
    ) -> Result<()> {
        self.tree
            .as_ref()
            .context(STARTUP_TREE_UNAVAILABLE_CONTEXT)?
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .record_explicit_bootstrap_exec_failure(failure)
    }
}
