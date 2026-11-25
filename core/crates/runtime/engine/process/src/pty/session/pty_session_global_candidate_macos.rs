use super::macos_candidate::CandidateEvaluation;
use super::macos_contract::{CapabilityProofError, CapabilityProofResult};
use anyhow::Result;

pub(super) fn fenced_global_candidate<G, P>(
    mut generation_matches: G,
    proof_matches: P,
) -> Result<CandidateEvaluation>
where
    G: FnMut() -> Result<bool>,
    P: FnOnce() -> CapabilityProofResult<bool>,
{
    if !matches!(generation_matches(), Ok(true)) {
        return Ok(CandidateEvaluation::Deferred);
    }
    let proof_matches = match proof_matches() {
        Ok(matches) => matches,
        Err(
            CapabilityProofError::BatchBudgetExhausted(_)
            | CapabilityProofError::DescriptorScanDeferred(_),
        ) => return Ok(CandidateEvaluation::Retry),
        Err(
            CapabilityProofError::CandidateLocalQuery(_)
            | CapabilityProofError::ScanBudgetExceeded(_),
        ) => return Ok(CandidateEvaluation::Deferred),
        Err(error) => return Err(error.into()),
    };
    if !matches!(generation_matches(), Ok(true)) {
        return Ok(CandidateEvaluation::Deferred);
    }
    Ok(CandidateEvaluation::Complete(proof_matches))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/pty_session_discovery_fence_tests.rs"]
mod tests;
