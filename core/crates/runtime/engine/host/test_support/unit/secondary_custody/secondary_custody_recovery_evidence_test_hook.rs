use crate::pty_custody::CustodyHolderObservation;
use std::cell::RefCell;
use std::collections::VecDeque;

const HOOK_OCCUPIED_DETAIL: &str = "recovery evidence observation hook is already installed";

thread_local! {
    static OBSERVATIONS: RefCell<VecDeque<CustodyHolderObservation>> =
        const { RefCell::new(VecDeque::new()) };
}

pub(in crate::host) struct GuardianObservationGuard;

pub(in crate::host) fn install_guardian_observations_for_test(
    observations: impl IntoIterator<Item = CustodyHolderObservation>,
) -> GuardianObservationGuard {
    OBSERVATIONS.with_borrow_mut(|slot| {
        assert!(slot.is_empty(), "{HOOK_OCCUPIED_DETAIL}");
        slot.extend(observations);
    });
    GuardianObservationGuard
}

pub(super) fn take_guardian_observation() -> Option<CustodyHolderObservation> {
    OBSERVATIONS.with_borrow_mut(VecDeque::pop_front)
}

impl Drop for GuardianObservationGuard {
    fn drop(&mut self) {
        OBSERVATIONS.with_borrow_mut(VecDeque::clear);
    }
}
