use super::super::super::inventory_stable::StableInventory;
use super::super::macos_liveness::MacosGlobalScanMode;
use super::super::macos_object_identity::MacosPtyObjectIdentity;
use super::super::macos_path::TtyPathState;
use super::super::macos_path_holders::path_holders;
use super::super::{CapabilityCandidates, CapabilityScanResult, PtySessionProof};
use super::available::{
    available_candidates, available_fence_stable, capture_holder_identities, finish_available_scan,
    map_path_holder_error, require_stable_available_fence,
};
use anyhow::Result;

pub(super) fn scan(
    inventory: &StableInventory,
    proof: &PtySessionProof,
    before_path: MacosPtyObjectIdentity,
    mode: MacosGlobalScanMode,
) -> Result<CapabilityScanResult> {
    let expected_path = proof.pty_object_identity()?;
    let holder_pids = path_holders(
        &proof.tty_path,
        super::super::super::constants::MAX_PTY_PROOF_CANDIDATES,
    )
    .map_err(map_path_holder_error)?;
    let Some(holders) = capture_holder_identities(&holder_pids) else {
        return Ok(finish_available_scan(CapabilityCandidates::default(), true));
    };
    let mut result = available_candidates(inventory, proof, &holders, mode)?;
    let after_pids = path_holders(
        &proof.tty_path,
        super::super::super::constants::MAX_PTY_PROOF_CANDIDATES,
    )
    .ok();
    let after = after_pids.as_ref().and_then(capture_holder_identities);
    let after_path = match proof.tty_path_state() {
        Ok(TtyPathState::Available(identity)) => Some(identity),
        Ok(TtyPathState::Released) | Err(_) => None,
    };
    let stable = available_fence_stable(
        expected_path,
        before_path,
        &holder_pids,
        &holders,
        after_path,
        after_pids.as_ref(),
        after.as_ref(),
    );
    require_stable_available_fence(&mut result, stable);
    Ok(result)
}
