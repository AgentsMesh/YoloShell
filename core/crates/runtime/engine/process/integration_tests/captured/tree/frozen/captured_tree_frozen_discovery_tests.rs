use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_snapshot::install_repeating_test_inventory;
use crate::process::inventory_test_support::{TestProcess, entry};
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{
    CapturedProcessTree, FreezePoll, FreezingProcessTree, LiveProcessIdentity,
    MacosProcessCapabilityIssuer, PTY_SESSION_CAPABILITY_ENV, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;
use std::time::Instant;

const TEST_CAPABILITY: &str = "frozen-fork-global-scan";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const ACK_TIMEOUT_DETAIL: &str = "frozen fork acknowledgement did not complete";
const FREEZE_TIMEOUT_DETAIL: &str = "fresh frozen discovery epoch did not settle";
const PREMATURE_FREEZE_DETAIL: &str = "unresolved fork bypassed fresh discovery";

#[test]
fn frozen_fork_requires_two_global_epochs_and_a_fresh_event_fence() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLUMNS,
            pixel_width: TERMINAL_PIXEL_SIZE,
            pixel_height: TERMINAL_PIXEL_SIZE,
        })
        .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut root = TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        TEST_CAPABILITY,
        &writer,
    );
    let mut escaped = TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        TEST_CAPABILITY,
        &writer,
    );
    let root_live = LiveProcessIdentity::capture(root.identity)
        .unwrap()
        .unwrap();
    let escaped_live = LiveProcessIdentity::capture(escaped.identity)
        .unwrap()
        .unwrap();
    reader.verify_writer_holder(&root_live).unwrap();
    reader.verify_writer_holder(&escaped_live).unwrap();
    drop(writer);
    let inventory = vec![
        entry(root.identity, u32::default(), root.identity.pid),
        entry(escaped.identity, u32::default(), escaped.identity.pid),
    ];
    let _inventory = install_repeating_test_inventory(inventory);
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    proof
        .install_process_capability(Arc::clone(&reader))
        .unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    assert!(tree.events_are_gated_for_test());
    tree.authorize_destructive_for_test();
    cover_discovery(&mut tree);
    assert!(tree.identities().contains(&escaped.identity));
    tree.inject_unresolved_root_fork_for_test();

    let mut freezing = wait_for_acknowledgement(tree.begin_force());
    assert_eq!(
        root_live.execution_state().unwrap(),
        ProcessExecutionState::Stopped
    );
    assert_eq!(
        escaped_live.execution_state().unwrap(),
        ProcessExecutionState::Stopped
    );
    freezing.tree.events.queue_test_fork(root_live.clone());
    assert!(freezing.tree.events.requires_reconciliation());
    assert!(freezing.tree.events.has_unresolved_fork());
    freezing = wait_for_acknowledgement(freezing);
    let frozen = wait_for_fresh_freeze(freezing);
    reader.verify_writer_holder(&root_live).unwrap();
    reader.verify_writer_holder(&escaped_live).unwrap();

    drop(frozen);
    root.terminate();
    escaped.terminate();
}

fn cover_discovery(tree: &mut CapturedProcessTree) {
    tree.begin_frozen_discovery();
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !tree.discovery_epoch.coverage_is_complete() {
        tree.refresh().unwrap();
        assert!(Instant::now() < deadline, "{FREEZE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Covered);
    assert!(tree.capability_scan_pending);
}

fn wait_for_acknowledgement(mut freezing: FreezingProcessTree) -> FreezingProcessTree {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        freezing = match freezing.poll().unwrap() {
            FreezePoll::Pending(next) => next,
            FreezePoll::Frozen(_) => panic!("{PREMATURE_FREEZE_DETAIL}"),
        };
        if !freezing.tree.events.has_unresolved_fork() {
            assert_eq!(freezing.tree.discovery_epoch, DiscoveryScanEpoch::Pending);
            assert!(freezing.tree.lineage_integrity_lost);
            return freezing;
        }
        assert!(Instant::now() < deadline, "{ACK_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_fresh_freeze(mut freezing: FreezingProcessTree) -> super::super::FrozenProcessTree {
    let deadline = Instant::now() + TEST_TIMEOUT;
    let mut observed_fresh_coverage = false;
    loop {
        match freezing.poll().unwrap() {
            FreezePoll::Pending(next) => {
                observed_fresh_coverage |= next.tree.discovery_epoch.coverage_is_complete();
                freezing = next;
            }
            FreezePoll::Frozen(frozen) => {
                assert!(observed_fresh_coverage);
                return frozen;
            }
        }
        assert!(
            Instant::now() < deadline,
            "{FREEZE_TIMEOUT_DETAIL}: {:?}",
            (
                freezing.tree.discovery_epoch,
                freezing.tree.lineage_settled,
                freezing.tree.lineage_integrity_lost,
                freezing.tree.capability_scan_pending,
                freezing.tree.pty_scan.writer_is_present(),
                freezing.tree.events.has_unresolved_fork(),
                freezing.tree.positive_closure_coverage(&[]),
            )
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}
