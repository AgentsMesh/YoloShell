#[cfg(test)]
use super::super::inventory_snapshot::InventoryEntry;
use super::super::inventory_stable::StableInventory;
#[cfg(test)]
use super::CapabilityCandidates;
#[cfg(test)]
use super::CapabilityScanAdvance;
#[cfg(test)]
use super::macos::EnvironmentCapability;
use super::macos_candidate::exclude_root;
#[cfg(test)]
use super::macos_candidate::{
    CANDIDATE_GENERATION_FENCE_CONTEXT, SINGLE_TEST_CANDIDATE, match_environment_if_tty,
    match_tty_if_capability,
};
#[cfg(test)]
use super::macos_candidate::{CandidateEvaluation, fenced_candidate};
#[cfg(test)]
use super::macos_contract::{CapabilityProofError, CapabilityProofResult, CapabilityScanBudget};
use super::macos_liveness::{
    MacosCapabilityLivenessDecision, MacosGlobalScanMode, complete_authorized_scan,
    global_scan_mode, observe, terminal,
};
use super::macos_path::TtyPathState;
#[cfg(test)]
use super::macos_path_holders::PathHolderError;
use super::macos_released::{scan_escaped, scan_released};
use super::{CapabilityScanResult, CapabilityScanStatus, PtyCapabilityScanState, PtySessionProof};
use anyhow::Result;
#[cfg(test)]
use std::collections::HashMap;
use std::collections::HashSet;
#[path = "../pty_session_available_scan_macos.rs"]
mod available;
#[path = "../pty_session_available_path_scan_macos.rs"]
mod available_path;
#[cfg(test)]
use available::{evaluate_candidates, map_path_holder_error};

impl StableInventory {
    pub(super) fn macos_capability_candidates(
        &self,
        proof: &PtySessionProof,
        state: &PtyCapabilityScanState,
        exact_priorities: &HashSet<super::super::LiveProcessIdentity>,
        excluded: &HashSet<super::super::LiveProcessIdentity>,
        minimum_start_time: u64,
        root_lineage: u64,
        root_exited: bool,
        scan_escaped_processes: bool,
    ) -> Result<CapabilityScanResult> {
        let liveness = observe(proof, state.liveness, root_exited);
        let (liveness, require_pending, reset_released) = match liveness {
            MacosCapabilityLivenessDecision::Complete(state) => {
                return Ok(terminal(CapabilityScanStatus::Complete, state));
            }
            MacosCapabilityLivenessDecision::Pending(state) => {
                return Ok(terminal(CapabilityScanStatus::Pending, state));
            }
            MacosCapabilityLivenessDecision::Scan {
                state,
                require_pending,
                reset_released,
            } => (state, require_pending, reset_released),
        };
        let released = reset_released.then(|| state.released.restart_preserving_matches());
        let released = released.as_ref().unwrap_or(&state.released);
        let excluded = exclude_root(excluded, exact_priorities, root_lineage, root_exited);
        let mode = global_scan_mode(proof, liveness);
        let freeze_round = root_exited || scan_escaped_processes;
        let released_transferable = matches!(
            mode,
            MacosGlobalScanMode::TransferableWriterReleased
                | MacosGlobalScanMode::ReleasedDiscoveryRetained
        );
        let mut result = if proof.is_discovery()
            || scan_escaped_processes
            || root_exited
            || released_transferable
        {
            scan_escaped(
                self,
                proof,
                released,
                minimum_start_time,
                root_lineage,
                mode,
                freeze_round,
                exact_priorities,
                &excluded,
            )?
        } else {
            match proof.tty_path_state()? {
                TtyPathState::Available(before_path) => {
                    available_path::scan(self, proof, before_path, mode)?
                }
                TtyPathState::Released => scan_released(
                    self,
                    proof,
                    released,
                    minimum_start_time,
                    root_lineage,
                    mode,
                    &excluded,
                )?,
            }
        };
        require_writer_settlement(&mut result, require_pending);
        let liveness = complete_authorized_scan(liveness, result.candidates.status);
        Ok(CapabilityScanResult {
            candidates: result.candidates,
            advance: result.advance.with_macos_liveness(liveness),
            scheduling_status: result.scheduling_status,
            coverage_status: result.coverage_status,
        })
    }
}

fn require_writer_settlement(result: &mut CapabilityScanResult, required: bool) {
    if required {
        result.candidates.status = CapabilityScanStatus::Pending;
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/pty_session_available_fence_tests.rs"]
mod available_fence_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/candidate/pty_session_candidate_match_tests.rs"]
mod candidate_match_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
use candidate_match_tests::candidate_matches;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/proof/pty_session_proof_budget_tests.rs"]
mod budget_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/scan/pty_session_scan_coverage_tests.rs"]
mod coverage_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/scan/pty_session_scan_holder_tests.rs"]
mod holder_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/pty_session_process_support_tests.rs"]
pub(super) mod process_test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/scan/pty_session_scan_process_tests.rs"]
mod process_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/top/pty_session_released_top_level_process_tests.rs"]
mod released_top_level_process_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/spawn/pty_session_spawn_fence_process_tests.rs"]
mod spawn_fence_process_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/spawn/pty_session_spawn_fence_tests.rs"]
mod spawn_fence_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/proof/pty_session_proof_tests.rs"]
mod tests;
