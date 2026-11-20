use super::super::macos_contract::{CapabilityProofError, CapabilityProofResult};
use anyhow::Result;

pub(in super::super) const CANDIDATE_GENERATION_FENCE_CONTEXT: &str =
    "fence PTY proof candidate after a local query failure";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) enum CandidateEvaluation {
    Complete(bool),
    Deferred,
    Retry,
}

pub(in super::super) fn fenced_candidate<G, P>(
    generation_matches: G,
    proof_matches: P,
) -> Result<CandidateEvaluation>
where
    G: FnMut() -> Result<bool>,
    P: FnOnce() -> CapabilityProofResult<bool>,
{
    fenced_with_generation_loss(
        generation_matches,
        proof_matches,
        CandidateEvaluation::Complete(false),
    )
}

fn fenced_with_generation_loss<G, P>(
    mut generation_matches: G,
    proof_matches: P,
    generation_loss: CandidateEvaluation,
) -> Result<CandidateEvaluation>
where
    G: FnMut() -> Result<bool>,
    P: FnOnce() -> CapabilityProofResult<bool>,
{
    if !generation_matches()? {
        return Ok(generation_loss);
    }
    let proof_matches = match proof_matches() {
        Ok(matches) => matches,
        Err(
            CapabilityProofError::BatchBudgetExhausted(_)
            | CapabilityProofError::DescriptorScanDeferred(_),
        ) => return Ok(CandidateEvaluation::Retry),
        Err(CapabilityProofError::CandidateLocalQuery(proof_error)) => {
            return match generation_matches() {
                Ok(false) => Ok(generation_loss),
                Ok(true) => Err(CapabilityProofError::candidate(proof_error).into()),
                Err(error) => Err(error.context(format!(
                    "{CANDIDATE_GENERATION_FENCE_CONTEXT}: {proof_error}"
                ))),
            };
        }
        Err(error) => return Err(error.into()),
    };
    if !generation_matches()? {
        return Ok(generation_loss);
    }
    Ok(CandidateEvaluation::Complete(proof_matches))
}
