use super::*;
use crate::process::CapturedProcessTree;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION_ARGUMENT: &str = "30";
const EXIT_OBSERVATION_TIMEOUT: Duration = Duration::from_secs(3);
const EXIT_OBSERVATION_POLL_INTERVAL: Duration = Duration::from_millis(10);
const STALE_START_TIME_OFFSET: u64 = 1;

#[test]
fn waitid_observes_exit_without_reaping_owned_child() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION_ARGUMENT),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let mut observer = WaitidRootExitObserver::new(root);
    let stale_root = ProcessIdentity {
        start_time: root.start_time.wrapping_add(STALE_START_TIME_OFFSET),
        ..root
    };
    assert!(
        WaitidRootExitObserver::new(stale_root)
            .poll_exited_unreaped(&authority)
            .is_err()
    );
    assert!(observer.poll_exited_unreaped(&authority).unwrap().is_none());
    root.signal(libc::SIGKILL).unwrap();
    let deadline = Instant::now() + EXIT_OBSERVATION_TIMEOUT;
    while observer.poll_exited_unreaped(&authority).unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "waitid did not observe child exit"
        );
        std::thread::sleep(EXIT_OBSERVATION_POLL_INTERVAL);
    }
    assert!(
        observer.poll_exited_unreaped(&authority).unwrap().is_some(),
        "waitid did not preserve the unreaped exit observation"
    );
    let outcome = crate::worker::pty_child_reaper::wait_direct_child(root.pid);
    assert!(outcome.status.is_some());
    assert!(
        observer.poll_exited_unreaped(&authority).is_err(),
        "waitid still owned an observation after explicit reap"
    );
}

#[test]
fn wait_authority_rejects_a_child_from_another_tree() {
    let mut tree_child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION_ARGUMENT),
    );
    let tree_root = ProcessIdentity::capture(tree_child.id()).unwrap();
    let tree = CapturedProcessTree::capture(tree_root).unwrap();
    let foreign_child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION_ARGUMENT),
    );
    let foreign_root = ProcessIdentity::capture(foreign_child.id()).unwrap();

    let foreign_child = Box::new(foreign_child) as crate::process::OwnedPtyChild;
    let rejected = RootWaitAuthority::prove_owned_child(&foreign_child, &tree).is_err();

    foreign_root.signal(libc::SIGKILL).unwrap();
    let foreign_outcome = crate::worker::pty_child_reaper::wait_direct_child(foreign_root.pid);
    tree_root.signal(libc::SIGKILL).unwrap();
    tree_child.wait().unwrap();
    assert!(foreign_outcome.status.is_some());
    assert!(rejected);
}
