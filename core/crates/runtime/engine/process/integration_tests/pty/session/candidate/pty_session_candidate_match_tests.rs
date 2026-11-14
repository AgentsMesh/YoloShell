use super::super::macos_contract::CapabilityProofResult;
use super::{CandidateEvaluation, fenced_candidate};
use anyhow::Result;

pub(super) fn candidate_matches<G, P>(generation_matches: G, proof_matches: P) -> Result<bool>
where
    G: FnMut() -> Result<bool>,
    P: FnOnce() -> CapabilityProofResult<bool>,
{
    match fenced_candidate(generation_matches, proof_matches)? {
        CandidateEvaluation::Complete(matches) => Ok(matches),
        CandidateEvaluation::Deferred | CandidateEvaluation::Retry => Ok(false),
    }
}
