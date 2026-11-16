use super::{BootstrapExecFailureFrame, ReleasedBootstrapGate};
use crate::process::{BootstrapExecTransitionAuthority, ExplicitBootstrapExecFailureProof};
use crate::protocol::{OptionalFrameRead, read_optional_frame};
use anyhow::{Context, Result, ensure};
use tokio::time::{Instant, timeout_at};

pub(crate) const EMPTY_EXEC_FAILURE_DETAIL: &str =
    "bootstrap exec failure detail must not be empty";

#[derive(Debug)]
pub(crate) enum BootstrapExecConfirmation {
    Confirmed,
    ExplicitFailure(ExplicitBootstrapExecFailure),
}

pub(crate) struct ExplicitBootstrapExecFailure {
    pub(crate) proof: ExplicitBootstrapExecFailureProof,
    pub(crate) detail: Box<str>,
}

impl ReleasedBootstrapGate {
    pub(crate) async fn confirm_exec(
        self,
        authority: BootstrapExecTransitionAuthority,
    ) -> Result<BootstrapExecConfirmation> {
        self.confirm_exec_until(authority, Instant::now() + super::BOOTSTRAP_GATE_TIMEOUT)
            .await
    }

    pub(crate) async fn confirm_exec_until(
        mut self,
        authority: BootstrapExecTransitionAuthority,
        deadline: Instant,
    ) -> Result<BootstrapExecConfirmation> {
        let result = timeout_at(
            deadline,
            read_optional_frame::<_, BootstrapExecFailureFrame>(&mut self.0),
        )
        .await
        .context(super::BOOTSTRAP_GATE_TIMEOUT_CONTEXT)?;
        match result {
            Ok(OptionalFrameRead::CleanEof) => {
                self.1.confirm(deadline).await?;
                authority.confirm_exec()?;
                Ok(BootstrapExecConfirmation::Confirmed)
            }
            Err(error) => Err(error).context(super::BOOTSTRAP_EXEC_FAILURE_CONTEXT),
            Ok(OptionalFrameRead::Frame(failure)) => {
                ensure!(!failure.detail.is_empty(), EMPTY_EXEC_FAILURE_DETAIL);
                let proof = authority.confirm_explicit_failure()?;
                Ok(BootstrapExecConfirmation::ExplicitFailure(
                    ExplicitBootstrapExecFailure {
                        proof,
                        detail: failure.detail.into_boxed_str(),
                    },
                ))
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn confirm_exec_before(
        self,
        authority: BootstrapExecTransitionAuthority,
        deadline: Instant,
    ) -> Result<BootstrapExecConfirmation> {
        self.confirm_exec_until(authority, deadline).await
    }
}

impl ExplicitBootstrapExecFailure {
    pub(crate) fn detail(&self) -> &str {
        &self.detail
    }

    pub(crate) fn into_process_proof(self) -> ExplicitBootstrapExecFailureProof {
        self.proof
    }
}

impl std::fmt::Debug for ExplicitBootstrapExecFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExplicitBootstrapExecFailure")
            .field("detail", &self.detail)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_gate_confirmation_adversarial_tests.rs"]
mod adversarial_tests;
