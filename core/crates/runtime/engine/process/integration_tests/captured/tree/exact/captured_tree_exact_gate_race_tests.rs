use super::exact_gate_fixture::ExactGateFixture;
use super::{exact_stopped_closure, inspect};
use crate::process::KillCommitPoll;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::live_identity::ProcessExecutionState;
use std::time::Instant;

const BETWEEN_CHECKS_CAPABILITY: &str = "exact-between-checks-gate-authority";
const STALE_COVERAGE_CAPABILITY: &str = "exact-stale-event-coverage";
const EXPECTED_REFREEZE_DETAIL: &str = "late inherited writer minted exact kill authority";
const EVENTLESS_WRITER_ACCEPTED_DETAIL: &str =
    "late writer without a process event bypassed the final fresh inspection";
const FRESH_COVERAGE_TIMEOUT_DETAIL: &str = "fresh exact event coverage did not recover";

#[test]
fn queued_inherited_writer_event_between_freeze_checks_forces_refreeze() {
    let mut fixture = ExactGateFixture::spawn(BETWEEN_CHECKS_CAPABILITY, true);
    let mut frozen = fixture.take_frozen();
    let source = frozen.tree.captured_targets()[usize::default()].0;
    let late = fixture.late_identity().unwrap();
    fixture.install_observed_inventory();
    frozen.tree.events.queue_quiescent_test_read();
    frozen.tree.events.queue_test_fork(source);

    let version = frozen.closure_version();
    let freezing = match frozen.kill_after_output_fence(version).unwrap() {
        KillCommitPoll::Refreeze(freezing) => freezing,
        KillCommitPoll::Pending(_) => panic!("{EXPECTED_REFREEZE_DETAIL}"),
    };

    assert_eq!(
        source.execution_state().unwrap(),
        ProcessExecutionState::Stopped
    );
    assert!(freezing.tree.identities().contains(&late));
    assert!(late.matches());
}

#[test]
fn eventless_late_writer_forces_a_fresh_commit_refreeze() {
    let mut fixture = ExactGateFixture::spawn(BETWEEN_CHECKS_CAPABILITY, true);
    let frozen = fixture.take_frozen();
    let late = fixture.late_identity().unwrap();
    fixture.install_observed_inventory();

    let version = frozen.closure_version();
    let freezing = match frozen.kill_after_output_fence(version).unwrap() {
        KillCommitPoll::Refreeze(freezing) => freezing,
        KillCommitPoll::Pending(_) => panic!("{EVENTLESS_WRITER_ACCEPTED_DETAIL}"),
    };

    assert!(freezing.tree.identities().contains(&late));
    assert!(late.matches());
}

#[test]
fn post_inventory_event_rejects_stale_coverage_then_fresh_scan_recovers() {
    let mut fixture = ExactGateFixture::spawn(STALE_COVERAGE_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    let source = tree.captured_targets()[usize::default()].0;
    fixture.install_observed_inventory();
    let _hook = tree.queue_fork_after_next_inventory(source);

    tree.refresh().unwrap();

    assert!(tree.lineage_integrity_lost);
    assert!(tree.events.has_unresolved_fork());
    assert!(exact_stopped_closure(&mut tree).unwrap().is_none());
    wait_for_fresh_authority(&mut tree);
}

fn wait_for_fresh_authority(tree: &mut crate::process::CapturedProcessTree) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        if inspect(tree).unwrap().is_some() {
            return;
        }
        assert!(Instant::now() < deadline, "{FRESH_COVERAGE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}
