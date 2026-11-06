use super::empty::empty_closure_can_report_gone;
use super::empty_eof_fixture::EmptyEofFixture;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree::freeze::{
    FreezeClosureCandidate, exact_stopped_candidate, inspect,
};
use crate::process::captured_tree::frozen_authority::FrozenClosureAuthority;
use crate::process::inventory_test_support::TestProcess;
use crate::process::{DescendantObservation, LiveProcessIdentity};

const UNEXPECTED_REFRESH_DETAIL: &str = "exact empty handoff entered a process inventory refresh";

#[test]
fn completed_coverage_allows_only_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();

    assert!(fixture.tree.discovery_epoch.coverage_is_complete());
    assert_eq!(
        fixture.tree.descendant_observation(),
        DescendantObservation::Indeterminate
    );
    assert!(!empty_closure_can_report_gone(
        fixture.tree.descendant_observation()
    ));
    assert!(handoff_is_authorized(&mut fixture));
}

#[test]
fn pending_capability_coverage_allows_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.discovery_epoch = DiscoveryScanEpoch::Pending;

    assert!(handoff_is_authorized(&mut fixture));
}

#[test]
fn exact_empty_handoff_does_not_wait_for_another_inventory_scan() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.fail_next_refresh(UNEXPECTED_REFRESH_DETAIL);

    assert!(matches!(
        inspect(&mut fixture.tree).unwrap(),
        Some(FreezeClosureCandidate::Ordinary(closure)) if closure.is_empty()
    ));
    assert!(
        fixture
            .tree
            .refresh()
            .unwrap_err()
            .to_string()
            .contains(UNEXPECTED_REFRESH_DETAIL)
    );
}

#[test]
fn exact_empty_handoff_rejects_a_fresh_post_inventory_event() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    let (_source, live) = fixture.register_source();
    fixture.tree.events.queue_test_fork(live);

    assert!(
        exact_stopped_candidate(&mut fixture.tree)
            .unwrap()
            .is_none()
    );
    assert!(fixture.tree.events.has_unresolved_fork());
}

#[test]
fn retained_fork_debt_denies_handoff_before_coverage_completes() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    fixture.tree.frozen_fork_reconciliation_required = true;

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn missing_fork_debt_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.events.acknowledge_root_exit_discovery_epoch();

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn live_unresolved_source_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    let (_source, live) = fixture.register_source();
    fixture.tree.events.inject_unresolved_fork(live);

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn registered_source_without_exact_gone_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    let (mut source, live) = fixture.register_source();
    fixture.tree.events.inject_unresolved_fork(live);
    source.terminate();

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn missing_event_gate_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.events.disable_gate_for_test();

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn post_inventory_event_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    let (_source, live) = fixture.register_source();
    fixture.tree.events.queue_test_fork(live);

    assert_handoff_is_denied(&mut fixture);
    assert!(
        fixture
            .tree
            .events
            .unresolved_fork_lineages_for_test()
            .contains(&live.lineage_id())
    );
}

#[test]
fn completed_capability_scan_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.capability_scan_pending = false;

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn unsettled_lineage_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.lineage_settled = false;

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn captured_member_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    let member = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(member.identity)
        .unwrap()
        .unwrap();
    fixture.tree.captured.insert(live, member.identity);

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn non_root_retiring_member_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    let member = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(member.identity)
        .unwrap()
        .unwrap();
    fixture.tree.retiring.insert(live);

    assert_handoff_is_denied(&mut fixture);
}

#[test]
fn unauthorized_output_fence_denies_the_kernel_eof_handoff() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.pty_master_released = true;

    assert_handoff_is_denied(&mut fixture);
}

fn assert_handoff_is_denied(fixture: &mut EmptyEofFixture) {
    assert_eq!(
        fixture.tree.descendant_observation(),
        DescendantObservation::Indeterminate
    );
    assert!(!handoff_is_authorized(fixture));
}

fn handoff_is_authorized(fixture: &mut EmptyEofFixture) -> bool {
    fixture
        .tree
        .frozen_closure_is_settled(FrozenClosureAuthority::OutputFencedEmpty)
}
