use super::*;
use std::io::Write;
use std::process::Stdio;

const OBSERVER_CHILD_SCRIPT: &str =
    "trap '' HUP; printf ready > \"$1\"; IFS= read -r command; exit 0";
const OBSERVER_EXIT_INPUT: &[u8] = b"exit\n";
const OBSERVER_PENDING_DETAIL: &str = "unbound Linux guard did not retain observer-only custody";
const OBSERVER_SETTLEMENT_DETAIL: &str = "unbound Linux guard did not settle after observed exit";
const NATURAL_EXIT_CODE: i32 = 0;

#[test]
fn unbound_linux_guard_cannot_force_a_hup_resistant_process_without_containment() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(CHILD_READY_CONTENT);
    let mut child = crate::process::spawn_waitable_test_child(
        Command::new(CHILD_SHELL)
            .args([
                CHILD_SCRIPT_ARGUMENT,
                OBSERVER_CHILD_SCRIPT,
                CHILD_SCRIPT_ROLE,
            ])
            .arg(&marker)
            .stdin(Stdio::piped()),
    );
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let mut exit_input = child.stdin.take().unwrap();
    let mut guard = StartupChildGuard::new(Box::new(child));
    let reaper = guard.fault_reaper.as_mut().unwrap();
    let completion = reaper.observe_startup_completion();
    let observer = reaper.observe_state_for_test();
    let deadline = std::time::Instant::now() + CHILD_TEST_TIMEOUT;
    while !marker.exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(CHILD_WAIT_INTERVAL);
    }
    assert!(guard.tree.is_none());
    drop(guard);
    let deadline = std::time::Instant::now() + CHILD_SETTLEMENT_TIMEOUT;
    while !observer
        .status()
        .is_some_and(|status| status.observer_only && status.custody_retained)
    {
        assert!(
            std::time::Instant::now() < deadline,
            "{OBSERVER_PENDING_DETAIL}: {:?}",
            observer.latest()
        );
        std::thread::sleep(CHILD_WAIT_INTERVAL);
    }
    assert!(identity.matches());
    assert!(
        observer
            .latest()
            .is_some_and(|snapshot| !snapshot.completion_published)
    );
    // The fixture cooperatively exits; the unbound guard never receives killing authority.
    exit_input.write_all(OBSERVER_EXIT_INPUT).unwrap();
    drop(exit_input);
    let outcome = completion
        .receive_timeout(CHILD_SETTLEMENT_TIMEOUT)
        .unwrap_or_else(|error| {
            panic!(
                "{OBSERVER_SETTLEMENT_DETAIL}: {error:?}; {:?}",
                observer.latest()
            )
        });
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    let status = outcome.status.unwrap();
    assert_eq!(status.signal(), None);
    assert_eq!(status.code(), Some(NATURAL_EXIT_CODE));
    assert!(!identity.matches());
}
