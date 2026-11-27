use super::pty_force_termination::{ForceOutput, ForceTerminationPoll};
use super::pty_missing_eof::MissingEofPoll;
use super::pty_output_progress::PtyOutputRecovery;
use super::pty_ownership::PtyOwnership;
use super::termination_timing::{CapabilitySettlementDeadline, CapabilitySettlementDeferred};
use super::termination_timing::{GRACEFUL_TERMINATION_POLL_INTERVAL, GRACEFUL_TERMINATION_TIMEOUT};
use anyhow::Result;
use std::time::Instant;

pub(crate) enum RootExitCleanup {
    Complete,
    Lost,
    Deferred(CapabilitySettlementDeferred),
}

pub(crate) fn cleanup_after_root_exit(
    ownership: &mut PtyOwnership,
    output: &mut PtyOutputRecovery,
) -> Result<RootExitCleanup> {
    if ownership.require_destructive_session().is_ok() {
        graceful_descendants(ownership, output)?;
    }
    let deadline = CapabilitySettlementDeadline::production(Instant::now());
    force_descendants(ownership, output, deadline)
}

pub(crate) fn force_descendants(
    ownership: &mut PtyOwnership,
    output: &mut PtyOutputRecovery,
    deadline: CapabilitySettlementDeadline,
) -> Result<RootExitCleanup> {
    let mut output_lost = false;
    loop {
        match ownership.poll_force(&mut ForceOutput::Reader(output))? {
            ForceTerminationPoll::Complete => {
                return Ok(if output_lost {
                    RootExitCleanup::Lost
                } else {
                    RootExitCleanup::Complete
                });
            }
            ForceTerminationPoll::Unsupported(_proof) => {
                ownership.enter_observer_only();
            }
            ForceTerminationPoll::Pending => {
                if !ownership.is_observer_only()
                    && ownership.poll_missing_eof(output, Instant::now())? == MissingEofPoll::Lost
                {
                    output_lost = true;
                }
            }
        }
        if deadline.expired_at(Instant::now()) {
            return Ok(RootExitCleanup::Deferred(CapabilitySettlementDeferred));
        }
        std::thread::sleep(GRACEFUL_TERMINATION_POLL_INTERVAL);
    }
}

#[cfg(test)]
pub(crate) fn force_descendants_for_test(
    ownership: &mut PtyOwnership,
    output: &mut PtyOutputRecovery,
    deadline: CapabilitySettlementDeadline,
) -> Result<RootExitCleanup> {
    force_descendants(ownership, output, deadline)
}

pub(crate) fn graceful_descendants(
    ownership: &mut PtyOwnership,
    output: &mut PtyOutputRecovery,
) -> Result<()> {
    let Some(tree) = ownership.tracking_tree_mut() else {
        return Ok(());
    };
    let deadline = Instant::now() + GRACEFUL_TERMINATION_TIMEOUT;
    while Instant::now() < deadline {
        tree.request_graceful_descendant_termination()?;
        output.progress()?;
        if tree.live_descendants()?.is_empty() {
            return Ok(());
        }
        std::thread::sleep(GRACEFUL_TERMINATION_POLL_INTERVAL);
    }
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_capability_settlement_tests.rs"]
mod tests;
