use crate::process::ProcessIdentity;
use crate::worker::startup::startup_guard::StartupChildGuard;
use crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;
use std::time::{Duration, Instant};

const PROCESS_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const PROCESS_SURVIVED_DETAIL: &str = "startup process survived cleanup";
const ROOT_PROCESS_ROLE: &str = "root";
const DESCENDANT_PROCESS_ROLE: &str = "descendant";
const INVALID_SESSION_ID: libc::pid_t = 0;

pub(super) fn bind_process_tree_inventory(
    guard: &StartupChildGuard,
    root: ProcessIdentity,
    descendant: ProcessIdentity,
) {
    let tree = guard.tree.as_ref().unwrap();
    tree.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .bind_repeating_process_inventory_for_test(&[
            (root, std::process::id(), process_session(root)),
            (descendant, root.pid, process_session(descendant)),
        ]);
}

pub(super) fn assert_process_exits(identity: ProcessIdentity) {
    assert_identity_exits(identity, DESCENDANT_PROCESS_ROLE);
}

pub(super) fn wait_for_root_exit(identity: ProcessIdentity) {
    assert_identity_exits(identity, ROOT_PROCESS_ROLE);
}

pub(super) fn assert_process_tree_exits(root: ProcessIdentity, descendant: ProcessIdentity) {
    assert_identity_exits(root, ROOT_PROCESS_ROLE);
    assert_identity_exits(descendant, DESCENDANT_PROCESS_ROLE);
}

fn assert_identity_exits(identity: ProcessIdentity, role: &str) {
    let deadline = Instant::now() + WORKER_TERMINATION_TIMEOUT;
    while identity.matches() {
        assert!(
            Instant::now() < deadline,
            "{PROCESS_SURVIVED_DETAIL}: {role}"
        );
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
}

fn process_session(identity: ProcessIdentity) -> u32 {
    let pid = i32::try_from(identity.pid).unwrap();
    let session = unsafe { libc::getsid(pid) };
    assert!(session > INVALID_SESSION_ID);
    u32::try_from(session).unwrap()
}
