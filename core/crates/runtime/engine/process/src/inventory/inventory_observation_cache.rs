use crate::process::inventory_stable::StableInventory;
use crate::process::pty_session_proof::{CapabilityScanResult, PtyCapabilityScanState};
use anyhow::Result;
use std::sync::Arc;

pub(in crate::process) fn capture(state: &PtyCapabilityScanState) -> Result<Arc<StableInventory>> {
    #[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
    let started_at = std::time::Instant::now();
    #[cfg(target_os = "macos")]
    if let Some(inventory) = state.frozen_inventory() {
        #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
        crate::process::pty_session_proof::macos_scan_diagnostic::record_inventory_capture(
            true,
            started_at.elapsed(),
        );
        return Ok(inventory);
    }
    let inventory = Arc::new(StableInventory::capture()?);
    #[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
    crate::process::pty_session_proof::macos_scan_diagnostic::record_inventory_capture(
        false,
        started_at.elapsed(),
    );
    Ok(inventory)
}

pub(in crate::process) fn retain(
    result: &mut CapabilityScanResult,
    inventory: &Arc<StableInventory>,
) {
    #[cfg(target_os = "macos")]
    result.retain_frozen_inventory(Arc::clone(inventory));
    #[cfg(not(target_os = "macos"))]
    let _ = (result, inventory);
}
