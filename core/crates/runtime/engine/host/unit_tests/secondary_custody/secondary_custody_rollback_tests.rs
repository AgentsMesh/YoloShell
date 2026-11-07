use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;
use std::time::Duration;

const ROLLBACK_DETAIL: &str = "rolled back PTY custody before activation";
const RELEASE_MISMATCH_DETAIL: &str = "Release does not match its exact offer";
const SETTLEMENT_TIMEOUT_DETAIL: &str = "process settlement timed out";
const POST_PREPARED_CANCEL_DETAIL: &str = "cancelled after the Prepared risk boundary";
const ROLLBACK_TEST_MARGIN: Duration = Duration::from_secs(2);
const UNEXPECTED_OPERATION_COMPLETION: &str = "custody operation completed unexpectedly";

#[path = "secondary_custody_rollback_after_drop_tests.rs"]
mod after_drop_tests;
#[path = "secondary_custody_rollback_live_tests.rs"]
mod live_tests;

#[tokio::test]
async fn exact_pre_activation_release_drops_platform_before_ack_without_fail_stop() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let aliases = fixture.take_pre_activation_local_aliases();
    let record = fixture.record.clone();
    let (server, ready, release, _finish) = fixture.serve_pre_activation_release(
        support::PreActivationReleaseIdentity::Exact,
        support::PreActivationReleaseResponse::ReadAcknowledgement,
    );
    let operation = fixture.host.ensure_secondary_custody(&record);
    tokio::pin!(operation);
    await_ready(ready, &mut operation).await;
    drop(aliases);
    let lease = current(&fixture.host, &record);
    assert!(lease.retains_platform());

    release.send(()).unwrap();
    let error = operation.await.unwrap_err();
    server.await.unwrap();

    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert!(error.to_string().contains(ROLLBACK_DETAIL));
    assert_eq!(lease.state(), SecondaryCustodyState::Released);
    assert!(!lease.retains_platform());
    assert!(!lease.is_wire_ready());
    assert!(lease.monitor_is_pending());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    assert!(current_optional(&fixture.host, &record).is_none());
}

#[tokio::test]
async fn forged_pre_activation_release_fail_stops_and_retains_exact_platform_custody() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let _aliases = fixture.take_pre_activation_local_aliases();
    let record = fixture.record.clone();
    let (server, ready, release, _finish) = fixture.serve_pre_activation_release(
        support::PreActivationReleaseIdentity::Forged,
        support::PreActivationReleaseResponse::DropWithoutReading,
    );
    let operation = fixture.host.ensure_secondary_custody(&record);
    tokio::pin!(operation);
    await_ready(ready, &mut operation).await;
    let lease = current(&fixture.host, &record);

    release.send(()).unwrap();
    let error = operation.await.unwrap_err();
    server.await.unwrap();

    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    assert!(error.to_string().contains(RELEASE_MISMATCH_DETAIL));
    assert_eq!(lease.state(), SecondaryCustodyState::Preparing);
    assert!(lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_some());
    assert!(Arc::ptr_eq(&lease, &current(&fixture.host, &record)));
}

#[tokio::test]
async fn unsettled_pre_activation_release_times_out_and_retains_platform_custody() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let _aliases = fixture.take_pre_activation_local_aliases();
    let record = fixture.record.clone();
    let (server, ready, release, finish) = fixture.serve_pre_activation_release(
        support::PreActivationReleaseIdentity::Exact,
        support::PreActivationReleaseResponse::HoldWithoutReading,
    );
    let operation = fixture.host.ensure_secondary_custody(&record);
    tokio::pin!(operation);
    await_ready(ready, &mut operation).await;
    let lease = current(&fixture.host, &record);

    release.send(()).unwrap();
    let error = tokio::time::timeout(
        crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT + ROLLBACK_TEST_MARGIN,
        &mut operation,
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert!(error.to_string().contains(SETTLEMENT_TIMEOUT_DETAIL));
    assert_eq!(lease.state(), SecondaryCustodyState::RollingBack);
    assert!(lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_some());
    finish.unwrap().send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn cancellation_after_exact_release_retains_platform_and_synchronously_fail_stops() {
    let mut fixture = support::MacosHandshakeFixture::with_live_child(None);
    let aliases = fixture.take_pre_activation_local_aliases();
    let record = fixture.record.clone();
    let (server, ready, release, finish) = fixture.serve_pre_activation_release(
        support::PreActivationReleaseIdentity::Exact,
        support::PreActivationReleaseResponse::HoldWithoutReading,
    );
    let lease = {
        let operation = fixture.host.ensure_secondary_custody(&record);
        tokio::pin!(operation);
        await_ready(ready, &mut operation).await;
        drop(aliases);
        let lease = current(&fixture.host, &record);
        release.send(()).unwrap();
        await_state_while_pending(
            &mut operation,
            &fixture.host,
            &lease,
            SecondaryCustodyState::RollingBack,
        )
        .await;
        lease
    };
    assert_eq!(lease.state(), SecondaryCustodyState::RollingBack);
    assert!(lease.retains_platform());
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(POST_PREPARED_CANCEL_DETAIL)
    );
    finish.unwrap().send(()).unwrap();
    server.await.unwrap();
}

async fn await_ready(
    ready: tokio::sync::oneshot::Receiver<()>,
    operation: &mut (impl std::future::Future<Output = Result<(), WorkerRpcError>> + Unpin),
) {
    tokio::select! {
        reached = ready => reached.unwrap(),
        result = operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
    }
}

async fn wait_for_state(
    host: &Host,
    lease: &SecondaryCustodyLease,
    expected: SecondaryCustodyState,
) {
    tokio::time::timeout(host.worker_ready_timeout, async {
        while lease.state() != expected {
            tokio::time::sleep(host.worker_ready_poll_interval).await;
        }
    })
    .await
    .unwrap();
}

async fn await_state_while_pending(
    operation: &mut (impl std::future::Future<Output = Result<(), WorkerRpcError>> + Unpin),
    host: &Host,
    lease: &SecondaryCustodyLease,
    expected: SecondaryCustodyState,
) {
    tokio::select! {
        () = wait_for_state(host, lease, expected) => {}
        result = operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
    }
}

fn current(host: &Host, record: &RuntimeRecord) -> Arc<SecondaryCustodyLease> {
    current_optional(host, record).unwrap()
}

fn current_optional(host: &Host, record: &RuntimeRecord) -> Option<Arc<SecondaryCustodyLease>> {
    host.secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(record))
}
