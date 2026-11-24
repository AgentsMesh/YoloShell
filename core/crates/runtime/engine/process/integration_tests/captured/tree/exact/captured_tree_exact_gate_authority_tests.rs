use super::exact_gate_fixture::ExactGateFixture;
use super::{FreezeClosureCandidate, exact_stopped_candidate, exact_stopped_closure};
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_snapshot::install_repeating_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{
    CapturedProcessTree, MacosProcessCapabilityIssuer, PTY_SESSION_CAPABILITY_ENV, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;
use std::time::Instant;

const UNAVAILABLE_GATE_CAPABILITY: &str = "exact-unavailable-gate-authority";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
pub(super) const EXACT_COVERAGE_REFRESH_LIMIT: usize = 16;
const COVERAGE_TIMEOUT_DETAIL: &str = "exact descriptor coverage did not settle";
const STOP_TIMEOUT_DETAIL: &str = "exact descriptor holder did not stop";
const INCOMPLETE_COVERAGE_CAPABILITY: &str = "exact-incomplete-coverage-authority";
const LIVE_SOURCE_CAPABILITY: &str = "exact-live-fork-source-authority";

#[test]
fn incomplete_global_coverage_uses_incremental_exact_positive_authority() {
    let mut fixture = ExactGateFixture::spawn(INCOMPLETE_COVERAGE_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    let expected = tree.captured_targets()[usize::default()].0;

    assert!(matches!(
        exact_stopped_candidate(&mut tree).unwrap(),
        Some(FreezeClosureCandidate::IncrementalExact(closure)) if closure == [expected]
    ));
}

#[test]
fn unresolved_source_inside_closure_denies_exact_stopped_authority() {
    let mut fixture = ExactGateFixture::spawn(LIVE_SOURCE_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    let source = tree.captured_targets()[usize::default()].0;
    tree.events.inject_unresolved_fork(source);
    tree.lineage_integrity_lost = true;

    assert!(tree.events.has_unresolved_fork());
    assert!(!tree.events.exact_closure_isolated(&[source]));
    assert!(exact_stopped_closure(&mut tree).unwrap().is_none());
}

#[test]
fn unavailable_event_gate_denies_exact_stopped_fast_authority() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut root = TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        UNAVAILABLE_GATE_CAPABILITY,
        &writer,
    );
    let mut escaped = TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        UNAVAILABLE_GATE_CAPABILITY,
        &writer,
    );
    drop(writer);
    let inventory = vec![
        entry(root.identity, NO_PARENT_PID, root.identity.pid),
        entry(escaped.identity, NO_PARENT_PID, escaped.identity.pid),
    ];
    let _inventory = install_repeating_test_inventory(inventory);
    let mut proof = PtySessionProof::capture_bound(
        pair.master.as_ref(),
        UNAVAILABLE_GATE_CAPABILITY.to_owned(),
    )
    .unwrap();
    proof
        .install_process_capability(Arc::clone(&reader))
        .unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    tree.begin_frozen_discovery();
    root.terminate();
    settle_exact_coverage(&mut tree);
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stop(&tree);
    tree.events.disable_gate_for_test();

    assert!(exact_stopped_closure(&mut tree).unwrap().is_none());
    escaped.terminate();
}

pub(super) fn settle_exact_coverage(tree: &mut CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !exact_coverage_is_settled(tree) {
        tree.refresh().unwrap();
        if exact_coverage_is_settled(tree) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "{COVERAGE_TIMEOUT_DETAIL}: {:#?}",
            tree.state_snapshot_for_test()
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn exact_coverage_is_settled(tree: &CapturedProcessTree) -> bool {
    let closure = tree
        .captured_targets()
        .into_iter()
        .map(|(identity, _durable)| identity)
        .collect::<Vec<_>>();
    tree.discovery_epoch.coverage_is_complete()
        && tree.lineage_settled
        && tree.exact_pty_closure_is_authorized(&closure)
}

pub(super) fn wait_for_stop(tree: &CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while tree.captured_targets().iter().any(|(identity, _durable)| {
        identity.execution_state().unwrap() != ProcessExecutionState::Stopped
    }) {
        assert!(Instant::now() < deadline, "{STOP_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

pub(super) fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
