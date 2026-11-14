use super::*;
use crate::process::{CapturedProcessTree, ProcessIdentity};
use crate::worker::pty_child_reaper::wait_direct_child;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const AUTHORITY_LOSS_COMPLETION_TIMEOUT: Duration = Duration::from_secs(1);
const FORBIDDEN_REFRESH_DETAIL: &str = "authority-loss reaper refreshed process inventory";
const INJECTED_REAPER_PANIC: &str = "injected transient fault reaper panic";
const INJECTED_POST_COMPLETION_FAULT: &str = "injected post-completion reaper fault";

#[test]
fn externally_reaped_child_completes_with_unknown_status() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    tree.fail_next_refresh_for_test(FORBIDDEN_REFRESH_DETAIL);
    root.signal(libc::SIGKILL).unwrap();
    assert!(wait_direct_child(root.pid).status.is_some());
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    let reaper = active_reaper(authority, tree, completion);
    let started = Instant::now();

    reaper.reap_with_driver(live_authority::reap_without_delay);

    assert!(started.elapsed() < AUTHORITY_LOSS_COMPLETION_TIMEOUT);
    let outcome = receiver
        .receive_timeout(AUTHORITY_LOSS_COMPLETION_TIMEOUT)
        .unwrap();
    assert!(outcome.status.is_none());
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::LostUnknown
    );
    let fault = outcome.fault.unwrap();
    assert_eq!(
        fault
            .root_cause()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::ECHILD)
    );
    assert!(!fault.to_string().contains(FORBIDDEN_REFRESH_DETAIL));
}

#[test]
fn transient_panic_preserves_authority_and_reaps_exactly_once() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    let reaper = active_reaper(authority, tree, completion);
    let mut panic_once = true;

    reaper.reap_with_driver(|ownership| {
        if std::mem::take(&mut panic_once) {
            panic!("{INJECTED_REAPER_PANIC}");
        }
        live_authority::reap_without_delay(ownership);
        ownership.remember_terminal(anyhow::anyhow!(INJECTED_POST_COMPLETION_FAULT));
        panic!("{INJECTED_POST_COMPLETION_FAULT}");
    });

    let outcome = receiver
        .receive_timeout(AUTHORITY_LOSS_COMPLETION_TIMEOUT)
        .unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert!(outcome.status.is_some());
    let fault = outcome.fault.unwrap();
    assert!(format!("{fault:#}").contains(INJECTED_REAPER_PANIC));
    assert!(matches!(
        receiver.receive_timeout(AUTHORITY_LOSS_COMPLETION_TIMEOUT),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout
            | std::sync::mpsc::RecvTimeoutError::Disconnected)
    ));
    let mut status = libc::c_int::default();
    let waited = unsafe { libc::waitpid(root.pid as libc::pid_t, &mut status, libc::WNOHANG) };
    assert_eq!(waited, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn mismatched_exit_proof_preserves_wait_authority_for_exact_retry() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let exact_proof = tree.root_exit_proof_for_test(root);
    let unrelated_root = ProcessIdentity::capture(std::process::id()).unwrap();
    let mismatched_proof = tree.root_exit_proof_for_test(unrelated_root);
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    let mut reaper = active_reaper(authority, tree, completion);

    assert!(!reaper.finish_reaped(mismatched_proof));
    assert!(reaper.custody().authority.is_some());
    assert!(reaper.custody_mut().root_exit_proof.take().is_some());
    assert!(reaper.failure.is_some());

    root.signal(libc::SIGKILL).unwrap();
    assert!(reaper.finish_reaped(exact_proof));
    let outcome = receiver
        .receive_timeout(AUTHORITY_LOSS_COMPLETION_TIMEOUT)
        .unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert!(outcome.status.is_some());
    assert!(outcome.fault.is_some());
}

fn active_reaper(
    authority: RootWaitAuthority,
    tree: CapturedProcessTree,
    completion: FaultReapCompletion,
) -> FaultReap {
    fault_reap_construction::fault_reap(
        authority,
        PtyProcessCustody::tracking(tree),
        PtyForceTermination::new(),
        FaultOutput::Reader(crate::worker::pty_output_progress::ReaderBackedPtyOutput::for_test()),
        crate::worker::pty_missing_eof::MissingEofDeadline::production(),
        None,
        None,
        completion,
        FaultReapStateObserver::default(),
    )
}

include!("branch_tests.rs");
include!("completion_failure_tests.rs");
include!("completion_containment_tests.rs");
include!("driver_adversarial_tests.rs");
include!("reservation_runtime_tests.rs");
#[path = "live_authority_containment_tests.rs"]
mod live_authority_containment_tests;
include!("proof_sync_tests.rs");
