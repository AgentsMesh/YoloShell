use super::reservation_claim::FaultReapClaim;
use super::{FaultReap, FaultReapReceiver, FaultReaperReservation};
use crate::worker::pty_force_termination::PtyForceTermination;
use crate::worker::pty_process_custody::{PtyProcessCustody, PtyProcessCustodySnapshot};
use anyhow::Result;
use std::sync::mpsc::{Receiver, sync_channel};
use std::time::Duration;

const HANDOFF_OBSERVATION_CHANNEL_CAPACITY: usize = 1;
const HELD_REAPER_CHANNEL_CAPACITY: usize = 1;
const HELD_REAPER_HANDOFF_TIMEOUT: Duration = Duration::from_secs(10);
const HANDOFF_OBSERVATION_DISCONNECTED_DETAIL: &str =
    "fault-reaper handoff observer disconnected unexpectedly";
const HELD_REAPER_HANDOFF_FAILURE_DETAIL: &str =
    "held fault-reaper reservation did not receive custody before the test deadline";
const HELD_REAPER_CUSTODY_MISSING_DETAIL: &str =
    "held fault-reaper reservation retained process custody";
const INJECTED_REAPER_DRIVER_PANIC_DETAIL: &str =
    "injected fault-reaper driver panic before force-state verification";

pub(crate) struct HeldFaultReaper {
    receiver: Receiver<FaultReapClaim>,
}

pub(crate) struct HeldFaultReap {
    ownership: FaultReap,
}

#[path = "reservation_retirement_test_support.rs"]
mod retirement;
pub(crate) use retirement::ReservationRetirement;

impl FaultReaperReservation {
    pub(crate) fn reserve_observed_for_test() -> Result<(Self, Receiver<PtyProcessCustodySnapshot>)>
    {
        let (observer, observations) = sync_channel(HANDOFF_OBSERVATION_CHANNEL_CAPACITY);
        let mut reservation = Self::reserve()?;
        reservation.handoff_observer = Some(observer);
        Ok((reservation, observations))
    }

    pub(crate) fn observe_handoff(&self, custody: &PtyProcessCustody) {
        self.observe_handoff_snapshot(custody.snapshot_for_test());
    }

    pub(crate) fn observe_handoff_snapshot(&self, snapshot: PtyProcessCustodySnapshot) {
        let Some(observer) = self.handoff_observer.as_ref() else {
            return;
        };
        let _ = crate::worker::supervision::contain(|| {
            observer
                .send(snapshot)
                .expect(HANDOFF_OBSERVATION_DISCONNECTED_DETAIL);
        });
    }

    pub(crate) fn reserve_held_for_test() -> Result<(Self, HeldFaultReaper)> {
        let state_observer = super::FaultReapStateObserver::default();
        let emergency =
            super::emergency_escrow::EmergencyEscrowProducer::reserve(state_observer.clone())?;
        let fallback = super::fallback::FaultReaperFallback::capture()?;
        let (sender, receiver) = sync_channel(HELD_REAPER_CHANNEL_CAPACITY);
        let reservation = Self {
            sender,
            fallback,
            emergency: Some(emergency),
            startup_completion: None,
            handoff_observer: None,
            state_observer,
            startup_accept_panics: false,
            startup_drive_panics: false,
            handoff_build_panics: false,
            handoff_dispatch_panics: false,
            retirement: None,
        };
        Ok((reservation, HeldFaultReaper { receiver }))
    }

    pub(crate) fn observe_state_for_test(&mut self) -> super::FaultReapStateObserver {
        self.state_observer.clone()
    }

    pub(crate) fn fill_primary_for_test(&self) {
        let control = super::fault_reap_construction::vacant_fault_reap(
            self.state_observer(),
            super::schedule::FaultReapSchedule::production(),
        );
        self.sender.send(FaultReapClaim::vacant(control)).unwrap();
    }
}

impl HeldFaultReaper {
    pub(crate) fn wait_for_handoff(self) -> HeldFaultReap {
        let claim = self
            .receiver
            .recv_timeout(HELD_REAPER_HANDOFF_TIMEOUT)
            .expect(HELD_REAPER_HANDOFF_FAILURE_DETAIL);
        let ownership = claim.take_required();
        HeldFaultReap { ownership }
    }
}

impl HeldFaultReap {
    pub(crate) fn custody_snapshot_for_test(&self) -> PtyProcessCustodySnapshot {
        self.ownership
            .custody()
            .processes
            .as_ref()
            .expect(HELD_REAPER_CUSTODY_MISSING_DETAIL)
            .snapshot_for_test()
    }

    pub(crate) fn is_observer_only_for_test(&self) -> bool {
        self.ownership.is_observer_only()
    }

    pub(crate) fn observe_completion_for_test(&mut self) -> FaultReapReceiver {
        let (completion, receiver) = super::FaultReapCompletion::owner_channel();
        self.ownership.replace_completion_for_test(completion);
        receiver
    }

    pub(crate) fn with_force_transaction_for_test<R>(
        &mut self,
        operation: impl FnOnce(&mut PtyForceTermination, &mut Option<PtyProcessCustody>) -> R,
    ) -> R {
        let custody = self.ownership.custody_mut();
        operation(&mut custody.force, &mut custody.processes)
    }

    pub(crate) fn resume_after_driver_panic_for_test(
        self,
        probe: impl FnOnce(&mut PtyForceTermination, &mut Option<PtyProcessCustody>),
    ) {
        let mut panic_pending = true;
        let mut probe = Some(probe);
        self.ownership.reap_with_driver(|ownership| {
            if std::mem::take(&mut panic_pending) {
                panic!("{INJECTED_REAPER_DRIVER_PANIC_DETAIL}");
            }
            if let Some(probe) = probe.take() {
                let custody = ownership.custody_mut();
                probe(&mut custody.force, &mut custody.processes);
            }
            super::live_authority::reap_without_delay(ownership);
        });
    }

    pub(crate) fn resume(self) {
        self.ownership.reap();
    }
}
