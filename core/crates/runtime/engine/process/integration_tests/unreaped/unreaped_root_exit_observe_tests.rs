use super::event_is_exit;
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{
    CapturedProcessTree, LiveProcessIdentity, ProcessIdentity, RootWaitAuthority,
};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const STATE_TIMEOUT: Duration = Duration::from_secs(5);
const STATE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const STATE_TIMEOUT_DETAIL: &str = "direct child did not reach the expected execution state";
const EXIT_TIMEOUT_DETAIL: &str = "waitid did not observe the direct child exit";
const UNKNOWN_EVENT_CODE: libc::c_int = libc::c_int::MAX;

struct ChildCleanup {
    root: ProcessIdentity,
    armed: bool,
}

impl Drop for ChildCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.root.signal(libc::SIGCONT);
            let _ = self.root.signal(libc::SIGKILL);
            let _ = crate::process::wait_direct_child(self.root.pid);
        }
    }
}

#[test]
fn stopped_and_continued_child_cannot_mint_exit_proof() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let live_root = tree.live_root();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let mut authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let mut cleanup = ChildCleanup { root, armed: true };
    assert!(authority.poll_exited_unreaped().unwrap().is_none());

    assert!(root.signal(libc::SIGSTOP).unwrap());
    wait_for_state(&live_root, |state| state == ProcessExecutionState::Stopped);
    assert!(authority.poll_exited_unreaped().unwrap().is_none());

    assert!(root.signal(libc::SIGCONT).unwrap());
    wait_for_state(&live_root, |state| state == ProcessExecutionState::Running);
    assert!(authority.poll_exited_unreaped().unwrap().is_none());

    assert!(root.signal(libc::SIGKILL).unwrap());
    let deadline = Instant::now() + STATE_TIMEOUT;
    let proof = loop {
        if let Some(proof) = authority.poll_exited_unreaped().unwrap() {
            break proof;
        }
        assert!(Instant::now() < deadline, "{EXIT_TIMEOUT_DETAIL}");
        std::thread::sleep(STATE_POLL_INTERVAL);
    };
    let waited = authority.wait_exited(proof).unwrap();
    assert!(authority.is_waited());
    let status = authority.reap(proof).unwrap();
    cleanup.armed = false;
    assert_eq!(status, waited);
    assert_eq!(status.signal(), Some(libc::SIGKILL));
}

#[test]
fn unknown_child_event_code_fails_closed() {
    assert!(event_is_exit(UNKNOWN_EVENT_CODE).is_err());
}

#[test]
fn only_terminal_child_event_codes_are_exit_proof() {
    for code in [libc::CLD_EXITED, libc::CLD_KILLED, libc::CLD_DUMPED] {
        assert!(event_is_exit(code).unwrap());
    }
    for code in [libc::CLD_STOPPED, libc::CLD_TRAPPED, libc::CLD_CONTINUED] {
        assert!(!event_is_exit(code).unwrap());
    }
}

fn wait_for_state(
    live_root: &LiveProcessIdentity,
    expected: impl Fn(ProcessExecutionState) -> bool,
) {
    let deadline = Instant::now() + STATE_TIMEOUT;
    loop {
        if expected(live_root.execution_state().unwrap()) {
            return;
        }
        assert!(Instant::now() < deadline, "{STATE_TIMEOUT_DETAIL}");
        std::thread::sleep(STATE_POLL_INTERVAL);
    }
}
