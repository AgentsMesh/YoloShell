use super::*;
use crate::worker::pty_fault_reaper::{FaultOutput, FaultReapCompletion};
use crate::worker::pty_output_progress::ReaderBackedPtyOutput;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const LIVE_AUTHORITY_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const LIVE_AUTHORITY_SLEEP_DURATION: &str = "30";
const LIVE_AUTHORITY_EXIT_TIMEOUT: Duration = Duration::from_secs(1);
const LIVE_AUTHORITY_REAP_INTERVAL: Duration = Duration::from_millis(1);
const LIVE_AUTHORITY_RETIRED_KILL_DELAY: Duration = Duration::from_millis(10);
const LIVE_AUTHORITY_WAIT_FAILURE: libc::pid_t = -1;

#[path = "completion_unwind_tests.rs"]
mod completion_unwind_tests;
#[path = "initial_signal_output_order_tests.rs"]
mod initial_signal_output_order;

struct LiveAuthorityCleanup {
    root: crate::process::ProcessIdentity,
    armed: bool,
}

impl Drop for LiveAuthorityCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.root.signal(libc::SIGCONT);
            let _ = self.root.signal(libc::SIGKILL);
            let _ = crate::process::wait_direct_child(self.root.pid);
        }
    }
}

#[test]
fn authority_free_settlement_distinguishes_observation_from_proven_loss() {
    let mut ownership = passive_ownership();

    initial_signal::request(&mut ownership);
    assert_eq!(
        poll_output_deadline(&mut ownership).unwrap(),
        MissingEofPoll::Pending
    );
    assert!(!finish_if_ready(&mut ownership));
    assert!(!ownership.completion().is_published());

    ownership.root_gone_without_wait = true;
    assert!(finish_if_ready(&mut ownership));
    assert!(ownership.completion().is_published());
    assert!(!ownership.has_unsettled_custody());
}

fn passive_ownership() -> FaultReap {
    super::super::fault_reap_construction::passive_fault_reap_for_test()
}

#[test]
fn live_authority_waits_for_an_exit_proof_before_reaping() {
    let (root, mut ownership) = waitable_ownership();
    let mut cleanup = LiveAuthorityCleanup { root, armed: true };

    assert!(!finish_if_ready(&mut ownership));
    assert!(ownership.custody().authority.is_some());
    assert!(!ownership.completion().is_published());

    assert!(root.signal(libc::SIGKILL).unwrap());
    let deadline = Instant::now() + LIVE_AUTHORITY_EXIT_TIMEOUT;
    while ownership.custody().root_exit_proof.is_none() {
        assert!(Instant::now() < deadline);
        observe_waitable_root_exit(&mut ownership);
        std::thread::yield_now();
    }
    assert!(!finish_if_ready(&mut ownership));
    retire_processes(&mut ownership);
    assert!(finish_if_ready(&mut ownership));
    assert!(ownership.completion().is_published());
    cleanup.armed = false;
}

#[test]
fn observed_exit_caches_wait_without_reviving_cancelled_reader_eof() {
    let lifecycle = Arc::new(crate::worker::ReaderLifecycle::default());
    let child_waited = Arc::new(AtomicBool::new(false));
    lifecycle.mark_eof_observed();
    assert!(lifecycle.kernel_eof_identity().is_some());
    lifecycle.cancel();
    assert!(lifecycle.is_cancelled());
    assert!(lifecycle.kernel_eof_identity().is_none());
    let output = ReaderBackedPtyOutput::observed_for_runtime(
        Arc::new(AtomicUsize::new(usize::default())),
        Arc::clone(&lifecycle),
        Arc::clone(&child_waited),
    );
    let (root, mut ownership) = waitable_ownership();
    ownership.custody_mut().output_progress = FaultOutput::Reader(output);
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    ownership.replace_completion_for_test(completion);
    let mut cleanup = LiveAuthorityCleanup { root, armed: true };

    assert!(root.signal(libc::SIGKILL).unwrap());
    let deadline = Instant::now() + LIVE_AUTHORITY_EXIT_TIMEOUT;
    while ownership.custody().root_exit_proof.is_none() {
        assert!(Instant::now() < deadline);
        observe_waitable_root_exit(&mut ownership);
        std::thread::yield_now();
    }

    assert!(child_waited.load(Ordering::Acquire));
    assert!(ownership.custody().authority.as_ref().unwrap().is_waited());
    assert!(ownership.custody().processes.is_some());
    assert!(!ownership.completion().is_published());
    assert!(lifecycle.is_cancelled());
    assert!(lifecycle.kernel_eof_identity().is_none());
    assert!(!finish_if_ready(&mut ownership));
    assert!(ownership.custody().processes.is_some());
    retire_processes(&mut ownership);
    assert!(finish_if_ready(&mut ownership));
    let outcome = receiver.receive();
    assert_eq!(outcome.status.unwrap().signal(), Some(libc::SIGKILL));
    cleanup.armed = false;
}

#[test]
fn retired_session_keeps_wait_authority_until_the_root_is_reaped() {
    let (root, mut ownership) = waitable_ownership();
    let mut cleanup = LiveAuthorityCleanup { root, armed: true };
    retire_processes(&mut ownership);
    assert!(root.signal(libc::SIGSTOP).unwrap());
    wait_for_stopped_child(root);
    let killer = std::thread::spawn(move || {
        std::thread::sleep(LIVE_AUTHORITY_RETIRED_KILL_DELAY);
        let _ = root.signal(libc::SIGKILL).unwrap();
    });

    reap_with_delays(
        &mut ownership,
        LIVE_AUTHORITY_REAP_INTERVAL,
        LIVE_AUTHORITY_REAP_INTERVAL,
    )
    .unwrap();

    killer.join().unwrap();
    assert!(ownership.completion().is_published());
    assert!(ownership.custody().authority.is_none());
    assert!(ownership.custody().processes.is_none());
    cleanup.armed = false;
}

fn retire_processes(ownership: &mut FaultReap) {
    ownership
        .custody_mut()
        .processes
        .as_mut()
        .unwrap()
        .tracking_mut()
        .retire_session_for_test();
}

fn waitable_ownership() -> (crate::process::ProcessIdentity, FaultReap) {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(LIVE_AUTHORITY_SLEEP_EXECUTABLE)).arg(LIVE_AUTHORITY_SLEEP_DURATION),
    );
    let root = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    let tree = crate::process::CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = crate::process::RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let mut ownership = passive_ownership();
    let custody = ownership.custody_mut();
    custody.authority = Some(authority);
    custody.processes = Some(crate::worker::pty_process_custody::PtyProcessCustody::tracking(tree));
    (root, ownership)
}

fn wait_for_stopped_child(root: crate::process::ProcessIdentity) {
    let mut status = libc::c_int::default();
    loop {
        let waited =
            unsafe { libc::waitpid(root.pid as libc::pid_t, &mut status, libc::WUNTRACED) };
        if waited == LIVE_AUTHORITY_WAIT_FAILURE
            && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
        {
            continue;
        }
        assert_eq!(waited, root.pid as libc::pid_t);
        assert!(libc::WIFSTOPPED(status));
        return;
    }
}
