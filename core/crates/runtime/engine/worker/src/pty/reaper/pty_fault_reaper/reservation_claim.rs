use super::reservation_claim_phase::{ClaimPhase, ClaimPhaseState};
use super::{FaultReap, OwnershipFaultHandoff};
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

pub(crate) const CLAIM_CUSTODY_MISSING_DETAIL: &str = "fault-reaper claim retained process custody";
pub(crate) const CLAIM_REAPER_PANIC_DETAIL: &str = "fault-reaper claim driver panicked";
#[cfg(test)]
pub(crate) const INJECTED_CLAIM_DRIVER_PANIC: &str = "injected fault-reaper claim driver panic";

#[derive(Clone)]
pub(crate) struct FaultReapClaim {
    pub(crate) shared: Arc<ClaimShared>,
}

pub(crate) struct ClaimShared {
    state: Mutex<ClaimState>,
    phase: ClaimPhaseState,
    ready: Condvar,
    #[cfg(test)]
    panic_before_drive: AtomicBool,
}

pub(crate) struct ClaimState {
    ownership: Option<FaultReap>,
}

impl FaultReapClaim {
    pub(crate) fn new(ownership: FaultReap) -> Self {
        let claim = Self {
            shared: Arc::new(ClaimShared {
                state: Mutex::new(ClaimState {
                    ownership: Some(ownership),
                }),
                phase: ClaimPhaseState::new(ClaimPhase::Ready),
                ready: Condvar::new(),
                #[cfg(test)]
                panic_before_drive: AtomicBool::new(false),
            }),
        };
        let _ = crate::worker::supervision::contain(|| claim.publish_handoff());
        claim
    }

    pub(crate) fn vacant(ownership: FaultReap) -> Self {
        Self {
            shared: Arc::new(ClaimShared {
                state: Mutex::new(ClaimState {
                    ownership: Some(ownership),
                }),
                phase: ClaimPhaseState::new(ClaimPhase::Vacant),
                ready: Condvar::new(),
                #[cfg(test)]
                panic_before_drive: AtomicBool::new(false),
            }),
        }
    }

    pub(crate) fn install(&self, handoff: &mut Option<OwnershipFaultHandoff>) -> bool {
        let mut state = self.lock();
        if state.ownership.is_none() || handoff.is_none() || !self.shared.phase.begin_install() {
            return false;
        }
        let Some(ownership) = state.ownership.as_mut() else {
            return false;
        };
        std::mem::swap(&mut ownership.handoff, handoff);
        ownership.failure = ownership
            .handoff
            .as_mut()
            .and_then(|handoff| handoff.failure.take());
        self.shared.phase.mark_installed();
        drop(state);
        let _ = crate::worker::supervision::contain(|| self.publish_handoff());
        self.shared.phase.mark_ready();
        self.shared.ready.notify_all();
        true
    }

    pub(crate) fn cancel(&self) {
        if self.shared.phase.cancel_vacant() {
            self.shared.ready.notify_all();
        }
    }

    #[cfg(test)]
    pub(crate) fn panic_before_drive_for_test(&self) {
        assert!(!self.shared.panic_before_drive.swap(true, Ordering::AcqRel));
    }

    pub(crate) fn reap(self) {
        loop {
            match catch_unwind(AssertUnwindSafe(|| self.drive_retained())) {
                Ok(()) => return,
                Err(payload) => {
                    self.remember_panic(payload);
                    std::thread::sleep(super::constants::PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL);
                }
            }
        }
    }

    pub(crate) fn publish_handoff(&self) {
        let state = self.lock();
        if let Some(ownership) = state
            .ownership
            .as_ref()
            .filter(|ownership| ownership.handoff.is_some())
        {
            ownership.publish_handoff();
        }
    }

    pub(crate) fn drive_retained(&self) {
        #[cfg(test)]
        if self.shared.panic_before_drive.swap(false, Ordering::AcqRel) {
            panic!("{INJECTED_CLAIM_DRIVER_PANIC}");
        }
        let mut state = self.wait_until_ready();
        if self.shared.phase.load() != ClaimPhase::Ready {
            return;
        }
        let Some(ownership) = state.ownership.as_mut() else {
            return;
        };
        ownership.reap_retained();
        if ownership.has_unsettled_custody() {
            return;
        }
        let settled = state.ownership.take();
        self.shared.phase.mark_settled();
        drop(state);
        let _ = crate::worker::supervision::contain(|| drop(settled));
    }

    pub(crate) fn remember_panic(&self, payload: Box<dyn std::any::Any + Send>) {
        let detail = crate::worker::supervision::panic_detail(payload);
        if let Some(ownership) = self
            .lock()
            .ownership
            .as_mut()
            .filter(|ownership| ownership.handoff.is_some())
        {
            ownership.remember(Err(anyhow::anyhow!(
                "{CLAIM_REAPER_PANIC_DETAIL}: {detail}"
            )));
        }
    }

    pub(crate) fn take(&self) -> Option<FaultReap> {
        let mut state = self.wait_until_ready();
        if self.shared.phase.load() != ClaimPhase::Ready {
            return None;
        }
        let ownership = state.ownership.take();
        if ownership.is_some() {
            self.shared.phase.mark_settled();
        }
        ownership
    }

    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, ClaimState> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn wait_until_ready(&self) -> std::sync::MutexGuard<'_, ClaimState> {
        let mut state = self.lock();
        while self.shared.phase.load().is_waiting() {
            state = self
                .shared
                .ready
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        state
    }

    #[cfg(test)]
    pub(crate) fn take_required(&self) -> FaultReap {
        self.take().expect(CLAIM_CUSTODY_MISSING_DETAIL)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/reservation_claim_tests.rs"]
mod tests;
