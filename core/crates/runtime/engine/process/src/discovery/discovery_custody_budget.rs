use super::CapturedProcessTree;
use super::constants::{PROCESS_INVENTORY_CAPTURE_ATTEMPTS, PROCESS_INVENTORY_RETRY_INTERVAL};
use super::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use anyhow::Result;
use std::time::Duration;

const DISCOVERY_OBSERVATION_REFRESH_EPOCHS: u32 = 1;
pub(super) const PRE_SIGNAL_REFRESH_EPOCHS: u32 = 1;
const FINALIZATION_SCAN_BUDGET_EPOCHS: u32 = 2;

pub(super) fn refresh_discovery_observation(tree: &mut CapturedProcessTree) -> Result<()> {
    (0..DISCOVERY_OBSERVATION_REFRESH_EPOCHS).try_for_each(|_| tree.refresh().map(|_| ()))
}

pub fn process_discovery_force_grace(
    settlement_epoch_grace: Duration,
    poll_interval: Duration,
) -> Duration {
    // Each force phase receives the scheduler window also used by direct-child reaping.
    settlement_epoch_grace
        .saturating_mul(force_scan_epochs())
        .saturating_add(
            PROCESS_INVENTORY_RETRY_INTERVAL.saturating_mul(inventory_retry_intervals()),
        )
        .saturating_add(poll_interval.saturating_mul(force_scan_epochs()))
}

fn force_scan_epochs() -> u32 {
    DISCOVERY_OBSERVATION_REFRESH_EPOCHS
        .saturating_add(PRE_SIGNAL_REFRESH_EPOCHS)
        .saturating_add(FINALIZATION_SCAN_BUDGET_EPOCHS)
}

fn inventory_retry_intervals() -> u32 {
    let retries = PROCESS_INVENTORY_CAPTURE_ATTEMPTS.saturating_sub(1);
    force_scan_epochs()
        .saturating_mul(count(STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE))
        .saturating_mul(count(retries))
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/discovery_custody_budget_inline_tests.rs"]
mod tests;
