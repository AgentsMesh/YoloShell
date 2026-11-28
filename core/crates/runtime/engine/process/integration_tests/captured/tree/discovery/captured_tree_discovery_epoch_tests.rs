use super::*;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::macos_process_capability::test_support;
use crate::process::{
    MacosProcessCapabilityChildWriter, MacosProcessCapabilityIssuer,
    MacosProcessCapabilityManagerReader, MacosProcessCapabilityObservation,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

const TEST_CAPABILITY: &str = "discovery-epoch-lineage-test";
const LIVENESS_TIMEOUT: Duration = Duration::from_secs(10);
const LIVENESS_POLL_INTERVAL: Duration = Duration::from_millis(1);
const STICKY_REFRESH_ATTEMPTS: usize = 3;
const LIVENESS_TIMEOUT_DETAIL: &str = "process capability watcher did not latch observation";

#[test]
fn observation_failure_cannot_rollback_premature_release_poison() {
    let mut root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        root.identity,
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let (reader, writer) = install_capability(&mut tree);
    drop(writer);
    wait_for_observation(
        &reader,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );
    tree.begin_discovery_cleanup_internal();
    let _failure = tree.fail_after_next_capability_observation();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let _live = install_test_inventory([vec![root_entry], vec![root_entry]]);
    assert!(tree.refresh().is_err());
    drop(_live);

    root.terminate();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    assert_sticky_after_empty_refreshes(
        &mut tree,
        &reader,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );
}

#[test]
fn observation_failure_cannot_rollback_protocol_poison() {
    let mut root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        root.identity,
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let (reader, writer) = install_capability(&mut tree);
    test_support::write_unexpected_byte(&writer).unwrap();
    wait_for_observation(&reader, MacosProcessCapabilityObservation::Indeterminate);
    tree.begin_discovery_cleanup_internal();
    let _failure = tree.fail_after_next_capability_observation();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let _live = install_test_inventory([vec![root_entry], vec![root_entry]]);
    assert!(tree.refresh().is_err());
    drop(_live);

    root.terminate();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    assert_sticky_after_empty_refreshes(
        &mut tree,
        &reader,
        MacosProcessCapabilityObservation::Indeterminate,
    );
    drop(writer);
}

#[test]
fn installed_monitor_latches_live_root_release_without_tree_polling() {
    let mut root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        root.identity,
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let (reader, writer) = install_capability(&mut tree);
    drop(writer);
    wait_for_observation(
        &reader,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );

    root.terminate();
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    assert_sticky_after_empty_refreshes(
        &mut tree,
        &reader,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );
}

fn install_capability(
    tree: &mut CapturedProcessTree,
) -> (
    Arc<MacosProcessCapabilityManagerReader>,
    MacosProcessCapabilityChildWriter,
) {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    tree.install_process_capability(reader.clone()).unwrap();
    assert_eq!(
        reader.liveness_observation(),
        Some(MacosProcessCapabilityObservation::WriterPresent)
    );
    (reader, writer)
}

fn assert_sticky_after_empty_refreshes(
    tree: &mut CapturedProcessTree,
    reader: &MacosProcessCapabilityManagerReader,
    expected: MacosProcessCapabilityObservation,
) {
    for _attempt in 0..STICKY_REFRESH_ATTEMPTS {
        let _gone = install_test_inventory([Vec::new(), Vec::new()]);
        tree.refresh().unwrap();

        assert!(tree.capability_scan_pending);
        assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
        assert_eq!(reader.liveness_observation(), Some(expected));
    }
}

fn wait_for_observation(
    reader: &MacosProcessCapabilityManagerReader,
    expected: MacosProcessCapabilityObservation,
) {
    let deadline = Instant::now() + LIVENESS_TIMEOUT;
    while Instant::now() < deadline {
        if reader.liveness_observation() == Some(expected) {
            return;
        }
        std::thread::sleep(LIVENESS_POLL_INTERVAL);
    }
    assert_eq!(
        reader.liveness_observation(),
        Some(expected),
        "{LIVENESS_TIMEOUT_DETAIL}"
    );
}
