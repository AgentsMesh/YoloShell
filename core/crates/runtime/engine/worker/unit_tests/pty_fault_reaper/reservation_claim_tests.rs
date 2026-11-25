use super::super::{
    FaultOutput, FaultReap, FaultReapCompletion, FaultReapReceiver, FaultReapStateObserver,
    OwnershipFaultHandoff, PtyForceTermination, PtyProcessCustody,
};
use super::*;
use crate::model::FaultReapProgressKind;
use crate::process::{CapturedProcessTree, ProcessIdentity, RootWaitAuthority};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{RecvTimeoutError, SyncSender, sync_channel};
use std::sync::{Arc, Barrier};
use std::time::Duration;

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const HANDOFF_MISSING_DETAIL: &str = "live fault-reap handoff is missing";
const STATUS_MISSING_DETAIL: &str = "fault-reap observer status is missing";
const PROGRESS_MISSING_DETAIL: &str = "fault-reap observer progress is missing";
const SINGLE_EVENT_CHANNEL_CAPACITY: usize = 1;
const CONCURRENT_DRIVER_COUNT: usize = 2;
const COORDINATOR_COUNT: usize = 1;
const DRIVER_BARRIER_PARTICIPANTS: usize = CONCURRENT_DRIVER_COUNT + COORDINATOR_COUNT;
const WAITPID_FAILURE: libc::pid_t = -1;
const DRIVER_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(10);
const PRE_CANCEL_OBSERVATION_TIMEOUT: Duration = Duration::from_millis(50);
const DUPLICATE_COMPLETION_OBSERVATION_TIMEOUT: Duration = Duration::from_millis(50);

#[test]
fn cancel_wakes_a_driver_waiting_for_vacant_custody() {
    let claim = vacant_claim(FaultReapStateObserver::default());
    let waiting = claim.clone();
    let (entered_sender, entered_receiver) = sync_channel(SINGLE_EVENT_CHANNEL_CAPACITY);
    let (done_sender, done_receiver) = sync_channel(SINGLE_EVENT_CHANNEL_CAPACITY);
    let driver = std::thread::spawn(move || {
        entered_sender.send(()).unwrap();
        waiting.reap();
        done_sender.send(()).unwrap();
    });
    entered_receiver
        .recv_timeout(DRIVER_SETTLEMENT_TIMEOUT)
        .unwrap();

    let premature = done_receiver.recv_timeout(PRE_CANCEL_OBSERVATION_TIMEOUT);
    claim.cancel();
    let completed = done_receiver.recv_timeout(DRIVER_SETTLEMENT_TIMEOUT);
    driver.join().unwrap();

    assert!(matches!(premature, Err(RecvTimeoutError::Timeout)));
    completed.unwrap();
}

#[test]
fn cancel_after_install_cannot_revoke_custody() {
    let (handoff, receiver, root) = live_handoff();
    let claim = vacant_claim(FaultReapStateObserver::default());
    let mut pending = Some(handoff);

    assert!(claim.install(&mut pending));
    assert!(pending.is_none());
    claim.cancel();
    let driver = spawn_driver(claim);

    let outcome = receiver.receive_timeout(DRIVER_SETTLEMENT_TIMEOUT).unwrap();
    driver.join().unwrap();
    assert!(outcome.status.is_some());
    assert_already_reaped(root);
}

#[test]
fn ready_driver_observes_published_handoff() {
    let observer = FaultReapStateObserver::default();
    let (handoff, receiver, root) = live_handoff();
    let claim = vacant_claim(observer.clone());
    let waiting = claim.clone();
    let observed = observer.clone();
    let (ready_sender, ready_receiver) = sync_channel(SINGLE_EVENT_CHANNEL_CAPACITY);
    let driver = std::thread::spawn(move || {
        let state = waiting.wait_until_ready();
        let phase = waiting.shared.phase.load();
        let status = observed.status();
        drop(state);
        ready_sender.send((phase, status)).unwrap();
        waiting.reap();
    });
    let mut pending = Some(handoff);

    assert!(claim.install(&mut pending));
    let (phase, status) = ready_receiver
        .recv_timeout(DRIVER_SETTLEMENT_TIMEOUT)
        .unwrap();
    assert!(phase == ClaimPhase::Ready);
    let progress = status
        .expect(STATUS_MISSING_DETAIL)
        .last_semantic_progress
        .expect(PROGRESS_MISSING_DETAIL);
    assert_eq!(progress.kind, FaultReapProgressKind::Handoff);

    let outcome = receiver.receive_timeout(DRIVER_SETTLEMENT_TIMEOUT).unwrap();
    driver.join().unwrap();
    assert!(outcome.status.is_some());
    assert_already_reaped(root);
}

#[test]
fn two_driver_clones_settle_and_publish_completion_once() {
    let (ownership, receiver, root) = live_reaper(FaultReapStateObserver::default());
    let claim = FaultReapClaim::new(ownership);
    let barrier = Arc::new(Barrier::new(DRIVER_BARRIER_PARTICIPANTS));
    let (done_sender, done_receiver) = sync_channel(CONCURRENT_DRIVER_COUNT);
    let first = spawn_barrier_driver(claim.clone(), Arc::clone(&barrier), done_sender.clone());
    let second = spawn_barrier_driver(claim, Arc::clone(&barrier), done_sender);

    barrier.wait();
    for _ in usize::default()..CONCURRENT_DRIVER_COUNT {
        done_receiver
            .recv_timeout(DRIVER_SETTLEMENT_TIMEOUT)
            .unwrap();
    }
    first.join().unwrap();
    second.join().unwrap();

    let outcome = receiver.receive_timeout(DRIVER_SETTLEMENT_TIMEOUT).unwrap();
    assert!(outcome.status.is_some());
    assert!(matches!(
        receiver.receive_timeout(DUPLICATE_COMPLETION_OBSERVATION_TIMEOUT),
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected)
    ));
    assert_already_reaped(root);
}

fn vacant_claim(observer: FaultReapStateObserver) -> FaultReapClaim {
    let control_plane = super::super::fault_reap_construction::vacant_fault_reap(
        observer,
        super::super::schedule::FaultReapSchedule::production(),
    );
    FaultReapClaim::vacant(control_plane)
}

fn live_handoff() -> (OwnershipFaultHandoff, FaultReapReceiver, ProcessIdentity) {
    let (mut reaper, receiver, root) = live_reaper(FaultReapStateObserver::default());
    let handoff = reaper.handoff.take().expect(HANDOFF_MISSING_DETAIL);
    (handoff, receiver, root)
}

fn live_reaper(
    observer: FaultReapStateObserver,
) -> (FaultReap, FaultReapReceiver, ProcessIdentity) {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(CHILD_EXECUTABLE)).arg(CHILD_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    let reaper = super::super::fault_reap_construction::fault_reap(
        authority,
        PtyProcessCustody::tracking(tree),
        PtyForceTermination::new(),
        FaultOutput::Reader(crate::worker::pty_output_progress::ReaderBackedPtyOutput::for_test()),
        crate::worker::pty_missing_eof::MissingEofDeadline::production(),
        None,
        None,
        completion,
        observer,
    );
    (reaper, receiver, root)
}

fn spawn_driver(claim: FaultReapClaim) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || claim.reap())
}

fn spawn_barrier_driver(
    claim: FaultReapClaim,
    barrier: Arc<Barrier>,
    done: SyncSender<()>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        barrier.wait();
        claim.reap();
        done.send(()).unwrap();
    })
}

fn assert_already_reaped(root: ProcessIdentity) {
    let mut status = libc::c_int::default();
    let waited = unsafe { libc::waitpid(root.pid as libc::pid_t, &mut status, libc::WNOHANG) };
    assert_eq!(waited, WAITPID_FAILURE);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
