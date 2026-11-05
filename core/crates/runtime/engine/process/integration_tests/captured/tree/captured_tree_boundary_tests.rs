use super::captured_targets;
use super::inventory_test_support::{NO_PARENT_PID, TestProcess};
use super::signal_authority::DestructiveSessionAccess;
use super::{
    CapturedProcessTree, LiveProcessIdentity, MacosProcessCapabilityIssuer,
    MacosProcessCapabilityObservation, ProcessIdentity, PtySessionProof,
    install_repeating_process_inventory_for_test, install_test_signal_hook,
};
use portable_pty::{PtySize, native_pty_system};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

const TEST_CAPABILITY: &str = "controller-boundary-capability";
const REFRESH_ATTEMPT_LIMIT: usize = 64;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const FIRST_TARGET_INDEX: usize = 0;
const NO_SIGNAL_DELIVERIES: usize = 0;

#[test]
fn parent_writer_is_never_admitted_while_monitor_refreshes() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let root = TestProcess::spawn();
    let proof =
        PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .transfer_for_test()
        .unwrap();
    tree.install_process_capability(Arc::clone(&reader))
        .unwrap();
    assert_eq!(
        reader.liveness_observation(),
        Some(MacosProcessCapabilityObservation::WriterPresent)
    );
    let controller = ProcessIdentity::capture(std::process::id()).unwrap();
    tree.bind_repeating_process_inventory_for_test(&[
        (root.identity, NO_PARENT_PID, root.identity.pid),
        (controller, NO_PARENT_PID, controller.pid),
    ]);

    let barrier = Arc::new(Barrier::new(2));
    let refresh_barrier = Arc::clone(&barrier);
    let refresh = std::thread::spawn(move || {
        refresh_barrier.wait();
        for _attempt in 0..REFRESH_ATTEMPT_LIMIT {
            tree.refresh().unwrap();
            if tree
                .state_snapshot_for_test()
                .capability_scheduling_complete
            {
                break;
            }
        }
        tree
    });
    barrier.wait();
    let tree = refresh.join().unwrap();

    assert!(
        tree.state_snapshot_for_test()
            .capability_scheduling_complete
    );
    assert!(!tree.identities().contains(&controller));
    drop(writer);
}

#[test]
fn excluded_anchor_cannot_expand_into_its_descendants() {
    let root = TestProcess::spawn();
    let excluded = TestProcess::spawn();
    let leaf = TestProcess::spawn();
    let _inventory = install_repeating_process_inventory_for_test(&[
        (root.identity, NO_PARENT_PID, root.identity.pid),
        (excluded.identity, root.identity.pid, excluded.identity.pid),
        (leaf.identity, excluded.identity.pid, leaf.identity.pid),
    ]);

    let mut tree =
        CapturedProcessTree::capture_lineage_excluding(root.identity, vec![excluded.identity])
            .unwrap();
    assert_eq!(tree.identities(), [root.identity]);
    tree.refresh().unwrap();
    assert_eq!(tree.identities(), [root.identity]);
    assert!(!tree.identities().contains(&excluded.identity));
    assert!(!tree.identities().contains(&leaf.identity));
}

#[test]
fn current_and_ancestor_targets_fail_before_signal_hooks() {
    assert_signal_for_test_rejects(controller_identity());
    assert_exact_sigkill_rejects(parent_identity());
}

fn assert_signal_for_test_rejects(durable: ProcessIdentity) {
    let calls = Arc::new(AtomicUsize::new(NO_SIGNAL_DELIVERIES));
    let observed = Arc::clone(&calls);
    let _hook = install_test_signal_hook(libc::SIGSTOP, FIRST_TARGET_INDEX, move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    let targets = [live_target(durable)];

    assert!(captured_targets::signal_for_test(&targets, libc::SIGSTOP).is_err());
    assert_eq!(calls.load(Ordering::SeqCst), NO_SIGNAL_DELIVERIES);
}

fn assert_exact_sigkill_rejects(durable: ProcessIdentity) {
    let calls = Arc::new(AtomicUsize::new(NO_SIGNAL_DELIVERIES));
    let observed = Arc::clone(&calls);
    let _hook = install_test_signal_hook(libc::SIGKILL, FIRST_TARGET_INDEX, move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    let targets = [live_target(durable)];
    let mut access = DestructiveSessionAccess::capture();
    access.authorize_for_test();

    assert!(captured_targets::signal_sigkill(access.require().unwrap(), &targets).is_err());
    assert_eq!(calls.load(Ordering::SeqCst), NO_SIGNAL_DELIVERIES);
}

fn live_target(durable: ProcessIdentity) -> (LiveProcessIdentity, ProcessIdentity) {
    (
        LiveProcessIdentity::capture(durable).unwrap().unwrap(),
        durable,
    )
}

fn controller_identity() -> ProcessIdentity {
    ProcessIdentity::capture(std::process::id()).unwrap()
}

fn parent_identity() -> ProcessIdentity {
    let pid = u32::try_from(unsafe { libc::getppid() }).unwrap();
    ProcessIdentity::capture(pid).unwrap()
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
