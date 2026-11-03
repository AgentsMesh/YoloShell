use super::*;
use crate::process::{ProcessIdentityRelation, ProcessIdentityRelationGuard};

const GUARDIAN_GONE_DETAIL: &str = "detached PTY custody guardian is proven gone";
const GUARDIAN_INDETERMINATE_DETAIL: &str =
    "detached PTY custody guardian identity is indeterminate";
const GUARDIAN_OBSERVATION_INTERVALS: u32 = 2;
const RETIRED_STATES: &[SecondaryCustodyState] = &[
    SecondaryCustodyState::ReleaseAckPending,
    SecondaryCustodyState::Released,
    SecondaryCustodyState::ReleaseUncertain,
    SecondaryCustodyState::Superseded,
];
const MONITORED_STATES: &[SecondaryCustodyState] = &[
    SecondaryCustodyState::Preparing,
    SecondaryCustodyState::RollingBack,
    SecondaryCustodyState::Activating,
    SecondaryCustodyState::Ready,
    SecondaryCustodyState::Disconnected,
    SecondaryCustodyState::Failed,
];

#[tokio::test]
async fn proven_gone_guardian_fail_stops_and_retains_live_runtime() {
    assert_guardian_failure(ProcessIdentityRelation::Gone, GUARDIAN_GONE_DETAIL).await;
}

#[tokio::test]
async fn indeterminate_guardian_fail_stops_and_retains_live_runtime() {
    assert_guardian_failure(
        ProcessIdentityRelation::Unknown,
        GUARDIAN_INDETERMINATE_DETAIL,
    )
    .await;
}

#[tokio::test]
async fn terminal_lease_wins_guardian_failure_race() {
    for &state in RETIRED_STATES {
        assert_terminal_race(state).await;
    }
}

#[tokio::test]
async fn non_retired_states_continue_guardian_observation() {
    for &state in MONITORED_STATES {
        assert_guardian_observed(state).await;
    }
}

async fn assert_guardian_failure(relation: ProcessIdentityRelation, detail: &str) {
    let mut fixture = support::MacosHandshakeFixture::with_live_child(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    let _guardian = install_guardian_relation(&fixture, relation);

    wait_until(&fixture.host, || {
        fixture.host.request_drain.fail_stop_detail().is_some()
    })
    .await;

    assert_eq!(lease.state(), SecondaryCustodyState::Failed);
    assert!(!lease.is_ready());
    assert!(lease.retains_platform());
    assert!(!lease.settlement_is_active());
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(detail)
    );
    assert!(Arc::ptr_eq(&lease, &current(&fixture.host, &record)));
    release.send(()).unwrap();
    server.await.unwrap();
}

async fn assert_terminal_race(terminal: SecondaryCustodyState) {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    lease.set_state(terminal);
    let _guardian = install_guardian_relation(&fixture, ProcessIdentityRelation::Gone);

    assert!(
        super::super::super::monitor::guardian_fault(Arc::downgrade(&lease))
            .await
            .is_ok()
    );

    tokio::time::sleep(crate::host::PROCESS_EXIT_POLL_INTERVAL * GUARDIAN_OBSERVATION_INTERVALS)
        .await;

    assert_eq!(lease.state(), terminal);
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    release.send(()).unwrap();
    server.await.unwrap();
}

async fn assert_guardian_observed(state: SecondaryCustodyState) {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    lease.set_state(state);
    let _guardian = install_guardian_relation(&fixture, ProcessIdentityRelation::Gone);

    assert!(
        super::super::super::monitor::guardian_fault(Arc::downgrade(&lease))
            .await
            .is_err()
    );
    release.send(()).unwrap();
    server.await.unwrap();
}

pub(super) fn install_guardian_relation(
    fixture: &support::MacosHandshakeFixture,
    relation: ProcessIdentityRelation,
) -> ProcessIdentityRelationGuard {
    let custody: crate::pty_custody::CustodyDurableRecord = fixture
        .host
        .namespace
        .read_json(&fixture.host.paths.pty_custody(fixture.record.terminal_id))
        .unwrap();
    let process = custody.armed_lease().unwrap().guardian.unwrap().process();
    crate::process::install_test_process_identity_relation(
        process.pid,
        process.start_time,
        relation,
    )
}
