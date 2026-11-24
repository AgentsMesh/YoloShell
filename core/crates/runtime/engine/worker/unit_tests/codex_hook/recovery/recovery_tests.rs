use super::recovery_test_support::RecoveryFixture;
use crate::agent::hooks::ProcessState;
use crate::codex_hooks::{CodexRetirementDisposition, CodexRetirementState};
use crate::process::ProcessIdentityObservation;

const NO_ROUTES: usize = 0;
const SINGLE_ROUTE: usize = 1;
const TWO_ROUTES: usize = 2;
const INITIAL_ROUTE_REVISION: u64 = 1;
const SUCCESSOR_ROUTE_REVISION: u64 = 2;
const LEXICALLY_FIRST_INVOCATION_VALUE: u128 = 1;

#[test]
fn matching_admission_recovers_a_committed_live_route() {
    let fixture = RecoveryFixture::new();
    let (router, recovered) = fixture.try_recover(ProcessIdentityObservation::Matching);

    recovered.unwrap();
    assert_eq!(router.len().unwrap(), SINGLE_ROUTE);
    let lease = router.lease_committed(&fixture.identity).unwrap();
    assert_eq!(
        lease.entry().runtime().state().unwrap().unwrap().process,
        ProcessState::Alive
    );
}

#[test]
fn recovery_preserves_commit_order_when_uuid_order_is_reversed() {
    let fixture = RecoveryFixture::new();
    let successor = fixture.additional_admission_with_invocation(uuid::Uuid::from_u128(
        LEXICALLY_FIRST_INVOCATION_VALUE,
    ));
    assert!(successor.record().request().identity.invocation_id < fixture.identity.invocation_id);

    let (router, recovered) = fixture.try_recover(ProcessIdentityObservation::Matching);

    recovered.unwrap();
    let routes = router.subscribe_committed().unwrap().snapshot;
    assert_eq!(routes.len(), TWO_ROUTES);
    assert_eq!(routes[0].revision, INITIAL_ROUTE_REVISION);
    assert_eq!(
        routes[0].entry.key().invocation_id(),
        fixture.identity.invocation_id
    );
    assert_eq!(routes[1].revision, SUCCESSOR_ROUTE_REVISION);
    assert_eq!(
        routes[1].entry.key().invocation_id(),
        successor.record().request().identity.invocation_id
    );
    assert!(router.reserve_publication_revision().unwrap() > SUCCESSOR_ROUTE_REVISION);
}

#[test]
fn unknown_process_fails_closed_without_installing_a_route() {
    let fixture = RecoveryFixture::new();
    let (router, recovered) = fixture.try_recover(ProcessIdentityObservation::Unknown);

    assert!(recovered.is_err());
    assert_eq!(router.len().unwrap(), NO_ROUTES);
    assert!(matches!(
        fixture.retirement_state(),
        CodexRetirementState::Unsealed
    ));
}

#[test]
fn gone_process_seals_settles_and_removes_the_route() {
    let fixture = RecoveryFixture::new();
    let (router, recovered) = fixture.try_recover(ProcessIdentityObservation::Gone);

    recovered.unwrap();
    assert_eq!(router.len().unwrap(), NO_ROUTES);
    assert_retired(&fixture);
}

#[test]
fn gone_process_quarantines_raw_only_intake_before_retirement() {
    let fixture = RecoveryFixture::new();
    fixture.persist_raw_start();
    let (router, recovered) = fixture.try_recover(ProcessIdentityObservation::Gone);

    recovered.unwrap();
    assert_eq!(router.len().unwrap(), NO_ROUTES);
    let scan = fixture.intake().scan().unwrap();
    assert!(scan.occurrences.is_empty());
    assert_eq!(scan.quarantined, SINGLE_ROUTE);
    assert_retired(&fixture);
}

#[test]
fn seal_only_crash_recovery_does_not_require_another_gone_observation() {
    let fixture = RecoveryFixture::new();
    fixture.publish_seal();
    let (router, recovered) = fixture.try_recover(ProcessIdentityObservation::Unknown);

    recovered.unwrap();
    assert_eq!(router.len().unwrap(), NO_ROUTES);
    assert_retired(&fixture);
}

#[test]
fn published_retirement_retry_revalidates_and_removes_the_route() {
    let fixture = RecoveryFixture::new();
    let (_, first) = fixture.try_recover(ProcessIdentityObservation::Gone);
    first.unwrap();
    assert_retired(&fixture);

    let (router, retry) = fixture.try_recover(ProcessIdentityObservation::Unknown);

    retry.unwrap();
    assert_eq!(router.len().unwrap(), NO_ROUTES);
    assert_retired(&fixture);
}

#[test]
fn retired_history_seeds_the_clock_and_keeps_the_next_restart_recoverable() {
    let fixture = RecoveryFixture::new();
    let (_, retired) = fixture.try_recover(ProcessIdentityObservation::Gone);
    retired.unwrap();
    assert_retired(&fixture);

    let (router, restarted) = fixture.try_recover(ProcessIdentityObservation::Unknown);
    restarted.unwrap();
    let next_revision = router.reserve_publication_revision().unwrap();
    assert_eq!(next_revision, SUCCESSOR_ROUTE_REVISION);
    let successor = fixture.additional_admission();
    assert_eq!(successor.record().route_revision(), next_revision);
    drop(router);

    let (router, second_restart) = fixture.try_recover(ProcessIdentityObservation::Matching);
    second_restart.unwrap();
    assert_eq!(router.len().unwrap(), SINGLE_ROUTE);
}

fn assert_retired(fixture: &RecoveryFixture) {
    let CodexRetirementState::Retired(retired) = fixture.retirement_state() else {
        panic!("Codex recovery did not publish retirement");
    };
    assert_eq!(
        retired
            .retirement()
            .record_for_test()
            .disposition_for_test(),
        CodexRetirementDisposition::Abandoned
    );
}
