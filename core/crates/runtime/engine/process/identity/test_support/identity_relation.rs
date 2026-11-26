use crate::ProcessIdentityObservation;
use std::cell::RefCell;

const MISSING_OVERRIDE: &str = "process identity test override is missing";

#[derive(Clone, Copy)]
struct Override {
    pid: u32,
    start_time: u64,
    observation: ProcessIdentityObservation,
}

thread_local! {
    static OVERRIDES: RefCell<Vec<Override>> = const { RefCell::new(Vec::new()) };
}

pub struct ProcessIdentityOverride {
    pid: u32,
    start_time: u64,
}

pub fn install_test_process_identity_relation(
    pid: u32,
    start_time: u64,
    observation: ProcessIdentityObservation,
) -> ProcessIdentityOverride {
    OVERRIDES.with_borrow_mut(|overrides| {
        overrides.push(Override {
            pid,
            start_time,
            observation,
        });
    });
    ProcessIdentityOverride { pid, start_time }
}

pub(crate) fn observation(pid: u32, start_time: u64) -> Option<ProcessIdentityObservation> {
    OVERRIDES.with_borrow(|overrides| {
        overrides
            .iter()
            .rev()
            .find(|value| value.pid == pid && value.start_time == start_time)
            .map(|value| value.observation)
    })
}

impl Drop for ProcessIdentityOverride {
    fn drop(&mut self) {
        OVERRIDES.with_borrow_mut(|overrides| {
            let index = overrides
                .iter()
                .rposition(|value| value.pid == self.pid && value.start_time == self.start_time)
                .expect(MISSING_OVERRIDE);
            overrides.remove(index);
        });
    }
}
