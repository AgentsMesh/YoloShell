use super::pty_force_termination::PtyForceTermination;
use super::pty_output_progress::{ExclusivePtyOutput, ReaderBackedPtyOutput};
use super::pty_ownership::ReapOutcome;
use super::pty_process_custody::PtyProcessCustody;
use super::supervision;
use super::terminal_reporting;
use crate::process::{LostRootWaitAuthority, RootWaitAuthority, WaitAuthorityLoss};
use anyhow::Result;
use std::sync::mpsc::SyncSender;

#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/pty_fault_reaper/adversarial_diagnostic.rs"]
mod adversarial_diagnostic;
#[path = "pty_fault_reaper/bootstrap_failure.rs"]
mod bootstrap_failure;
#[path = "pty_fault_reaper/completion.rs"]
mod completion;
#[path = "pty_fault_reaper/constants.rs"]
mod constants;
#[path = "pty_fault_reaper/driver.rs"]
mod driver;
#[cfg(test)]
use driver::{FAULT_REAPER_EARLY_RETURN_DETAIL, FAULT_REAPER_PANIC_DETAIL};
#[path = "pty_fault_reaper/emergency_escrow.rs"]
mod emergency_escrow;
#[path = "pty_fault_reaper/failure.rs"]
mod failure;
#[path = "pty_fault_reaper/fallback.rs"]
mod fallback;
#[path = "pty_fault_reaper/fault_reap_construction.rs"]
mod fault_reap_construction;
#[path = "pty_fault_reaper/fault_reap_custody.rs"]
mod fault_reap_custody;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/pty_fault_reaper/handoff_test_support.rs"]
mod handoff_test_support;
#[cfg(test)]
pub(crate) use handoff_test_support::HeldFaultReaper;
#[path = "pty_fault_reaper/inline.rs"]
mod inline;
#[path = "pty_fault_reaper/live_authority.rs"]
mod live_authority;
#[path = "pty_fault_reaper/output.rs"]
mod output;
#[path = "pty_fault_reaper/ownership_handoff.rs"]
mod ownership_handoff;
#[path = "pty_fault_reaper/ownership_handoff_transaction.rs"]
mod ownership_handoff_transaction;
#[path = "pty_fault_reaper/progress.rs"]
mod progress;
#[path = "pty_fault_reaper/reservation.rs"]
mod reservation;
#[path = "pty_fault_reaper/reservation_claim.rs"]
mod reservation_claim;
#[path = "pty_fault_reaper/reservation_claim_phase.rs"]
mod reservation_claim_phase;
#[path = "pty_fault_reaper/reservation_handoff.rs"]
mod reservation_handoff;
#[path = "pty_fault_reaper/reservation_runtime.rs"]
mod reservation_runtime;
#[path = "pty_fault_reaper/schedule.rs"]
mod schedule;
#[path = "pty_fault_reaper/state_observer.rs"]
mod state_observer;

pub(crate) use completion::FaultReapCompletion;
#[cfg(test)]
pub(crate) use completion::FaultReapReceiver;
pub(crate) use inline::{settle_inline, settle_inline_after_master_release};
pub(crate) use ownership_handoff::OwnershipFaultHandoff;
pub(crate) use reservation_handoff::StartupFaultHandoff;
pub(crate) use state_observer::FaultReapStateObserver;
#[cfg(test)]
pub(crate) use state_observer::FaultReapStateSnapshot;

pub(crate) struct FaultReaperReservation {
    pub(crate) sender: SyncSender<reservation_claim::FaultReapClaim>,
    pub(crate) fallback: fallback::FaultReaperFallback,
    pub(crate) emergency: Option<emergency_escrow::EmergencyEscrowProducer>,
    #[cfg(test)]
    pub(crate) startup_completion: Option<FaultReapCompletion>,
    #[cfg(test)]
    pub(crate) handoff_observer:
        Option<SyncSender<super::pty_process_custody::PtyProcessCustodySnapshot>>,
    pub(crate) state_observer: FaultReapStateObserver,
    #[cfg(test)]
    pub(crate) startup_accept_panics: bool,
    #[cfg(test)]
    pub(crate) startup_drive_panics: bool,
    #[cfg(test)]
    pub(crate) handoff_build_panics: bool,
    #[cfg(test)]
    pub(crate) handoff_dispatch_panics: bool,
    #[cfg(test)]
    pub(crate) retirement: Option<handoff_test_support::ReservationRetirement>,
}

pub(crate) struct FaultReap {
    handoff: Option<OwnershipFaultHandoff>,
    lost_wait_custody: Option<LostRootWaitAuthority>,
    failure: Option<anyhow::Error>,
    root_gone_without_wait: bool,
    schedule: schedule::FaultReapSchedule,
    state_observer: FaultReapStateObserver,
}

pub(crate) enum FaultOutput {
    Reader(ReaderBackedPtyOutput),
    Exclusive(ExclusivePtyOutput),
}

include!("pty_fault_reaper/fault_reap_methods.rs");
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/pty_fault_reaper/tests.rs"]
mod tests;
