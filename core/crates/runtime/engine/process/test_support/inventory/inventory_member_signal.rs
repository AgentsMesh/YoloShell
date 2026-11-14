use crate::process::{LiveProcessIdentity, ProcessGroupIdentity, ProcessIdentity};
use std::cell::Cell;
use std::rc::Rc;

#[path = "inventory_member_signal_observer.rs"]
pub(super) mod observer;

const SLEEP_PROGRAM: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const DELIVERY_COUNT_STEP: usize = 1;

pub(super) struct MemberProcess {
    child: std::process::Child,
    pub(super) identity: ProcessIdentity,
    pub(super) live: LiveProcessIdentity,
}

impl MemberProcess {
    pub(super) fn spawn() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            std::process::Command::new(SLEEP_PROGRAM).arg(SLEEP_DURATION),
        );
        Self::from_child(child)
    }

    pub(super) fn observe(directory: &std::path::Path) -> Self {
        let child = crate::process::spawn_waitable_test_child(&mut observer::command(directory));
        let process = Self::from_child(child);
        observer::wait_ready(directory);
        process
    }

    fn from_child(child: std::process::Child) -> Self {
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let live = LiveProcessIdentity::capture(identity).unwrap().unwrap();
        Self {
            child,
            identity,
            live,
        }
    }

    pub(super) fn group(&self, members: Vec<LiveProcessIdentity>) -> ProcessGroupIdentity {
        ProcessGroupIdentity::verified(
            crate::process::inventory::process_group(self.identity.pid)
                .unwrap()
                .unwrap(),
            members,
        )
    }
}

impl Drop for MemberProcess {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        let _ = self.child.wait();
    }
}

pub(super) fn observed() -> (Rc<Cell<usize>>, impl Drop) {
    let count = Rc::new(Cell::new(usize::default()));
    let observed = Rc::clone(&count);
    let guard = crate::process::install_test_live_signal_observer(move |_| {
        observed.set(observed.get() + DELIVERY_COUNT_STEP);
    });
    (count, guard)
}
