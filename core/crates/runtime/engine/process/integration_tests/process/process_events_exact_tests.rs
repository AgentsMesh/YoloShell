use super::ProcessEvents;
use crate::process::LiveProcessIdentity;
use crate::process::constants::EVENT_RECONCILIATION_LIMIT;
use crate::process::inventory_test_support::TestProcess;

const PUBLICATION_SETTLEMENT_DETAIL: &str = "exact-containment publication did not settle";

#[test]
fn unresolved_fork_debt_never_mints_exact_authority() {
    let (_source, _live, events) = authority_fixture();
    let allowed = events.unresolved_fork_versions();

    assert!(
        events
            .exact_lineage_containment_authority(&allowed)
            .is_none()
    );
}

#[test]
fn repeated_fork_keeps_exact_authority_unavailable() {
    let (_source, live, mut events) = authority_fixture();
    let allowed = events.unresolved_fork_versions();
    events.queue_test_fork_exit(live);

    assert!(!events.reconcile_inventory(std::iter::empty()));
    assert!(events.reconcile_inventory(std::iter::empty()));
    assert!(
        events
            .exact_lineage_containment_authority(&allowed)
            .is_none()
    );
}

#[test]
fn publication_debt_keeps_exact_authority_unavailable() {
    let (_source, _live, mut events) = authority_fixture();
    let allowed = events.unresolved_fork_versions();
    events.invalidate();

    assert!(
        events
            .exact_lineage_containment_authority(&allowed)
            .is_none()
    );
}

#[test]
fn event_gate_loss_keeps_exact_authority_unavailable() {
    let (_source, _live, mut events) = authority_fixture();
    let allowed = events.unresolved_fork_versions();
    events.disable_gate_for_test();

    assert!(
        events
            .exact_lineage_containment_authority(&allowed)
            .is_none()
    );
}

#[test]
fn exact_root_exit_event_is_bound_to_the_registered_generation() {
    let (source, live) = isolated_identity();
    let (_other_source, other) = isolated_identity();
    let mut events = ProcessEvents::new(live);
    events.queue_test_exit(live);

    assert!(
        events
            .exact_root_exit_proof(source.identity, other)
            .is_none()
    );
    assert!(
        events
            .exact_root_exit_proof(source.identity, live)
            .is_some()
    );
}

#[test]
fn a_new_unresolved_lineage_cannot_borrow_an_old_proof() {
    let (_source, live, mut events) = authority_fixture();
    let allowed = events.unresolved_fork_versions();
    let (_late_source, late) = isolated_identity();
    events.inject_gone_unresolved_fork(late);

    assert!(
        events
            .exact_lineage_containment_authority(&allowed)
            .is_none()
    );
    assert_eq!(
        events.unresolved_fork_lineages_for_test(),
        sorted_lineages([live, late])
    );
}

#[test]
fn gone_isolated_debt_can_commit_after_a_fresh_publication() {
    let (_source, live, mut events) = authority_fixture();
    events.inject_gone_unresolved_fork(live);
    settle_empty_publication(&mut events);
    let allowed = events.unresolved_fork_versions();
    let authority = events
        .exact_lineage_containment_authority(&allowed)
        .unwrap();

    settle_empty_publication(&mut events);

    assert!(events.acknowledge_exact_containment(&[], &authority));
    assert!(events.unresolved_fork_lineages_for_test().is_empty());
}

#[test]
fn partial_allowed_set_cannot_mint_exact_authority() {
    let (_source, live, mut events) = authority_fixture();
    let (_other_source, other) = isolated_identity();
    events.inject_gone_unresolved_fork(live);
    events.inject_gone_unresolved_fork(other);
    settle_empty_publication(&mut events);
    let mut allowed = events.unresolved_fork_versions();
    allowed.retain(|source, _version| *source == live.lineage_id());

    assert!(
        events
            .exact_lineage_containment_authority(&allowed)
            .is_none()
    );
}

#[test]
fn later_fork_version_revokes_prepared_exact_authority() {
    let (_source, live, mut events) = authority_fixture();
    events.inject_gone_unresolved_fork(live);
    settle_empty_publication(&mut events);
    let allowed = events.unresolved_fork_versions();
    let authority = events
        .exact_lineage_containment_authority(&allowed)
        .unwrap();
    let (_late_source, late) = isolated_identity();
    events.inject_gone_unresolved_fork(late);
    settle_empty_publication(&mut events);

    assert!(!events.acknowledge_exact_containment(&[], &authority));
    assert_eq!(
        events.unresolved_fork_lineages_for_test(),
        sorted_lineages([live, late])
    );
}

fn authority_fixture() -> (TestProcess, LiveProcessIdentity, ProcessEvents) {
    let (source, live) = isolated_identity();
    let mut events = ProcessEvents::new(live);
    events.inject_unresolved_fork(live);
    assert!(events.reconcile_inventory([live]));
    (source, live, events)
}

fn isolated_identity() -> (TestProcess, LiveProcessIdentity) {
    let process = TestProcess::spawn();
    let identity = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    (process, identity)
}

fn settle_empty_publication(events: &mut ProcessEvents) {
    for _attempt in 0..EVENT_RECONCILIATION_LIMIT {
        if events.reconcile_inventory(std::iter::empty()) {
            return;
        }
    }
    panic!("{PUBLICATION_SETTLEMENT_DETAIL}");
}

fn sorted_lineages(identities: [LiveProcessIdentity; 2]) -> Vec<u64> {
    let mut lineages = identities.map(|identity| identity.lineage_id()).to_vec();
    lineages.sort_unstable();
    lineages
}
