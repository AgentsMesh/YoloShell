use super::ProcessIdentityRelation;
use std::cell::RefCell;

#[derive(Clone, Copy)]
struct Override {
    pid: u32,
    start_time: u64,
    relation: ProcessIdentityRelation,
}

thread_local! {
    static OVERRIDES: RefCell<Vec<Override>> = const { RefCell::new(Vec::new()) };
}

pub struct Guard {
    pid: u32,
    start_time: u64,
    #[cfg(any(test, feature = "runtime-component-tests"))]
    _identity: yolo_process_identity::ProcessIdentityOverride,
}

pub fn install(pid: u32, start_time: u64, relation: ProcessIdentityRelation) -> Guard {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    let identity =
        yolo_process_identity::install_test_process_identity_relation(pid, start_time, relation);
    OVERRIDES.with_borrow_mut(|overrides| {
        overrides.push(Override {
            pid,
            start_time,
            relation,
        });
    });
    Guard {
        pid,
        start_time,
        #[cfg(any(test, feature = "runtime-component-tests"))]
        _identity: identity,
    }
}

pub(crate) fn relation(pid: u32, start_time: u64) -> Option<ProcessIdentityRelation> {
    OVERRIDES.with_borrow(|overrides| {
        overrides
            .iter()
            .rev()
            .find(|value| value.pid == pid && value.start_time == start_time)
            .map(|value| value.relation)
    })
}

impl Drop for Guard {
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

const MISSING_OVERRIDE: &str = "process identity test override is missing";
