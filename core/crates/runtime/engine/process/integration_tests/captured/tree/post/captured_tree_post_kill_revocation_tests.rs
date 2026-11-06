use super::super::super::termination::{ExactKillAuthority, RetainedExclusion};
use super::retained_tests::{RetainedFixture, assert_observation};
use super::*;

const REVOKED_DETAIL: &str = "fork debt restored revoked retained exclusion";
const EXACT_EMPTY_DETAIL: &str = "fork debt authorized retained exact-empty settlement";
const DEBT_NOT_CLEARED_DETAIL: &str = "fork debt did not clear after fresh reconciliation";

#[test]
fn unresolved_fork_irrevocably_revokes_retained_exclusion() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::ExactPositive);
    fixture.root.terminate();
    assert_observation(&mut fixture, PostKillObservation::Pending);
    fixture
        .tree
        .events
        .inject_gone_unresolved_fork(fixture.killed);

    assert_revoked_observation(&mut fixture);
    fixture.tree.events.acknowledge_root_exit_discovery_epoch();
    let _rebuilt = fixture.wait.observe(&mut fixture.tree).unwrap();
    assert!(
        !fixture.tree.events.has_unresolved_fork(),
        "{DEBT_NOT_CLEARED_DETAIL}"
    );
    assert_eq!(
        fixture.retained_exclusion(),
        RetainedExclusion::Revoked,
        "{REVOKED_DETAIL}"
    );
}

#[test]
fn frozen_fork_reconciliation_irrevocably_revokes_retained_exclusion() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::ExactPositive);
    fixture.root.terminate();
    assert_observation(&mut fixture, PostKillObservation::Pending);
    fixture.tree.frozen_fork_reconciliation_required = true;
    fixture
        .tree
        .force_next_frozen_fork_publication_gap_for_test();

    assert_revoked_observation(&mut fixture);
    let _rebuilt = fixture.wait.observe(&mut fixture.tree).unwrap();
    assert!(
        !fixture.tree.frozen_fork_reconciliation_required,
        "{DEBT_NOT_CLEARED_DETAIL}"
    );
    assert_eq!(
        fixture.retained_exclusion(),
        RetainedExclusion::Revoked,
        "{REVOKED_DETAIL}"
    );
}

fn assert_revoked_observation(fixture: &mut RetainedFixture) {
    let observed = fixture.wait.observe(&mut fixture.tree).unwrap();
    assert_ne!(
        observed,
        PostKillObservation::ExactEmpty,
        "{EXACT_EMPTY_DETAIL}"
    );
    assert_eq!(
        fixture.retained_exclusion(),
        RetainedExclusion::Revoked,
        "{REVOKED_DETAIL}"
    );
}
