use super::super::PtySessionProof;
use super::super::macos_candidate::CandidateEvaluation;
use super::super::macos_contract::CapabilityScanBudget;
use super::super::macos_descriptors::has_stable_tty_descriptor;
use super::super::macos_global_candidate::fenced_global_candidate;
use super::super::macos_liveness::MacosGlobalScanMode;
use super::universe::ReleasedCandidate;
use anyhow::Result;
#[cfg(test)]
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum GlobalCandidateContext {
    ReleasedTty,
    Escaped,
}

impl PtySessionProof {
    pub(super) fn matches_custody_process(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> super::super::macos_contract::CapabilityProofResult<bool> {
        let _scan_guard = self.descriptor_scan_guard()?;
        has_stable_tty_descriptor(pid, self.pty_object_identity()?, budget)
    }
}

pub(super) fn evaluate_global_candidate(
    proof: &PtySessionProof,
    candidate: ReleasedCandidate,
    budget: &mut CapabilityScanBudget,
    context: GlobalCandidateContext,
    mode: MacosGlobalScanMode,
) -> Result<CandidateEvaluation> {
    fenced_global_candidate(
        || candidate.live.try_matches(),
        || {
            match mode {
                MacosGlobalScanMode::CustodyPtyObject
                | MacosGlobalScanMode::RecoveryCustodyPtyObject => {
                    proof.matches_custody_process(candidate.live.pid(), budget)
                }
                MacosGlobalScanMode::TransferableWriterPresent => {
                    proof.observes_transferable_process(candidate.live.pid(), budget)
                }
                MacosGlobalScanMode::LineageEnvironment
                | MacosGlobalScanMode::PrematureBootstrapRelease => match context {
                    GlobalCandidateContext::ReleasedTty => {
                        proof.matches_released_process(candidate.live.pid(), budget)
                    }
                    GlobalCandidateContext::Escaped => {
                        proof.matches_escaped_process(candidate.live.pid(), budget)
                    }
                },
                MacosGlobalScanMode::ReleasedDiscoveryRetained => Ok(false),
                MacosGlobalScanMode::TransferableWriterReleased => {
                    // Negative observations advance scheduling only. Kernel EOF remains
                    // the global exclusion authority after writer release.
                    proof.observes_released_tty_process(candidate.live.pid(), budget)
                }
            }
            .map_err(|error| {
                error.with_candidate_identity(candidate.live.pid(), candidate.live.unique_id())
            })
        },
    )
}

#[cfg(test)]
const TEST_RETRY_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const TEST_RETRY_INTERVAL: Duration = Duration::from_millis(1);

#[cfg(test)]
pub(super) fn evaluate_test_candidate(
    proof: &PtySessionProof,
    candidate: ReleasedCandidate,
    budget: &mut CapabilityScanBudget,
    context: GlobalCandidateContext,
    mode: MacosGlobalScanMode,
) -> Result<CandidateEvaluation> {
    let deadline = Instant::now() + TEST_RETRY_TIMEOUT;
    loop {
        let result = evaluate_global_candidate(proof, candidate, budget, context, mode)?;
        if matches!(result, CandidateEvaluation::Complete(_)) {
            return Ok(result);
        }
        if result == CandidateEvaluation::Deferred
            && !matches!(candidate.live.try_matches(), Ok(true))
        {
            return Ok(result);
        }
        if Instant::now() >= deadline {
            return Ok(result);
        }
        *budget = CapabilityScanBudget::start_batch();
        std::thread::sleep(TEST_RETRY_INTERVAL);
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/root/pty_session_released_root_candidate_tests.rs"]
mod root_candidate_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/pty_session_released_candidate_tests.rs"]
mod tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/transferable/pty_session_transferable_candidate_tests.rs"]
mod transferable_tests;
