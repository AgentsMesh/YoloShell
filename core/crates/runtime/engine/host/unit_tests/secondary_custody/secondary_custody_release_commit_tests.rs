use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

#[path = "../../test_support/unit/secondary_custody/secondary_custody_release_commit_lock_order_test_support.rs"]
mod lock_order;

const UNCERTAIN_RELEASE_DETAIL: &str = "acknowledgement was cancelled after local release commit";
const INJECTED_POST_COMMIT_FAULT_DETAIL: &str = "injected post-commit monitor fault";

#[tokio::test]
async fn guardian_retirement_waits_without_holding_in_memory_custody_locks() {
    lock_order::assert_waiting_retirement_lock_order().await;
}

#[tokio::test]
async fn local_release_commit_retires_custody_before_acknowledgement() {
    let (mut fixture, lease, server, finish) = ready_fixture().await;
    let settlement = settled(&mut fixture, &lease).await;

    let pending = lease.commit_active_release(settlement).unwrap();

    assert_eq!(lease.state(), SecondaryCustodyState::ReleaseAckPending);
    assert!(!lease.is_ready());
    assert!(!lease.retains_platform());
    assert!(current(&fixture).is_none());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    pending.acknowledge();
    assert_eq!(lease.state(), SecondaryCustodyState::Released);
    finish.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn cancellation_after_local_release_commit_is_sticky_and_fail_stops() {
    let (mut fixture, lease, server, finish) = ready_fixture().await;
    let settlement = settled(&mut fixture, &lease).await;

    drop(lease.commit_active_release(settlement).unwrap());

    assert_eq!(lease.state(), SecondaryCustodyState::ReleaseUncertain);
    assert!(!lease.is_ready());
    assert!(!lease.retains_platform());
    assert!(current(&fixture).is_none());
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(UNCERTAIN_RELEASE_DETAIL)
    );
    failure::respond(
        Arc::downgrade(&lease),
        failure::CustodyMonitorFault::failed(INJECTED_POST_COMMIT_FAULT_DETAIL),
    )
    .await;
    assert_eq!(lease.state(), SecondaryCustodyState::ReleaseUncertain);
    assert!(current(&fixture).is_none());
    finish.send(()).unwrap();
    server.await.unwrap();
}

pub(super) async fn ready_fixture() -> (
    support::MacosHandshakeFixture,
    Arc<SecondaryCustodyLease>,
    tokio::task::JoinHandle<()>,
    tokio::sync::oneshot::Sender<()>,
) {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, finish) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = fixture
        .host
        .secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(&record))
        .unwrap();
    (fixture, lease, server, finish)
}

async fn settled<'lease>(
    fixture: &mut support::MacosHandshakeFixture,
    lease: &'lease SecondaryCustodyLease,
) -> settlement::ExactSecondaryCustodySettlement<'lease> {
    fixture.close_worker_aliases();
    tokio::time::timeout(
        crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT,
        settlement::wait_for_exact(lease),
    )
    .await
    .unwrap()
    .unwrap()
}

fn current(fixture: &support::MacosHandshakeFixture) -> Option<Arc<SecondaryCustodyLease>> {
    fixture
        .host
        .secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(&fixture.record))
}
