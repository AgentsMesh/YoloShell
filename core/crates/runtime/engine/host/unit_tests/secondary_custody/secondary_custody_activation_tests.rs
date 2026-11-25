use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;
use std::time::Duration;

const ACTIVATION_TEST_TIMEOUT: Duration = Duration::from_secs(5);
const POST_PREPARED_DETAIL: &str = "post-Prepared PTY custody failure";
const POST_PREPARED_CANCEL_DETAIL: &str = "cancelled after the Prepared risk boundary";
const HANDSHAKE_TIMEOUT_DETAIL: &str = "PTY secondary-custody handshake timed out";
const UNEXPECTED_OPERATION_COMPLETION: &str = "custody operation completed unexpectedly";

#[tokio::test]
async fn exact_worker_activation_starts_monitor_and_permits_availability() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_holding();

    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);

    assert!(lease.is_wire_ready());
    assert!(lease.is_ready());
    assert!(!lease.monitor_is_pending());
    assert_eq!(lease.state(), SecondaryCustodyState::Ready);

    let home = fixture.host.paths.home().to_path_buf();
    let namespace = fixture.host.paths.socket_dir().unwrap();
    let durable: CustodyDurableRecord = fixture
        .host
        .namespace
        .read_json(&fixture.host.paths.pty_custody(record.terminal_id))
        .unwrap();
    let guardian = durable.armed_lease().unwrap().guardian.unwrap().process();
    drop(lease);

    disconnect.send(()).unwrap();
    server.await.unwrap();
    drop(fixture);

    assert_eq!(
        guardian.observation(),
        crate::pty_custody::CustodyHolderObservation::ProvenGone
    );
    assert!(!namespace.exists());
    assert!(!home.exists());
}

#[tokio::test]
async fn missing_worker_activation_fails_closed_and_requests_host_fail_stop() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let server = fixture.serve_activation(support::ActivationFixture::Missing);

    let error = fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap_err();
    server.await.unwrap();
    let lease = current(&fixture.host, &record);

    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert!(!lease.is_wire_ready());
    assert!(!lease.is_ready());
    assert!(lease.monitor_is_pending());
    assert_eq!(lease.state(), SecondaryCustodyState::Preparing);
    assert!(lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_some());
}

#[tokio::test]
async fn forged_worker_activation_fails_exact_validation_and_requests_fail_stop() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let server = fixture.serve_activation(support::ActivationFixture::Forged);

    let error = fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap_err();
    server.await.unwrap();
    let lease = current(&fixture.host, &record);

    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    assert!(!lease.is_wire_ready());
    assert!(!lease.is_ready());
    assert!(lease.monitor_is_pending());
    assert_eq!(lease.state(), SecondaryCustodyState::Preparing);
    assert!(lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_some());
}

#[tokio::test]
async fn post_prepared_timeout_irreversibly_requests_host_fail_stop() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    fixture.host.worker_ready_timeout = ACTIVATION_TEST_TIMEOUT;
    let record = fixture.record.clone();
    let (server, prepared, release) = fixture.serve_after_prepared_holding();

    let operation = fixture.host.ensure_secondary_custody(&record);
    tokio::pin!(operation);
    tokio::select! {
        reached = prepared => reached.unwrap(),
        result = &mut operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
    }
    let error = operation.await.unwrap_err();
    let detail = fixture.host.request_drain.fail_stop_detail().unwrap();

    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert!(detail.contains(POST_PREPARED_DETAIL));
    assert!(detail.contains(HANDSHAKE_TIMEOUT_DETAIL));
    let lease = current(&fixture.host, &record);
    assert_eq!(lease.state(), SecondaryCustodyState::Preparing);
    assert!(lease.retains_platform());
    assert!(!lease.is_wire_ready());
    assert!(lease.monitor_is_pending());
    release.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn dropping_post_prepared_operation_synchronously_fail_stops_and_retains_custody() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, prepared, release) = fixture.serve_after_prepared_holding();

    {
        let operation = fixture.host.ensure_secondary_custody(&record);
        tokio::pin!(operation);
        tokio::select! {
            reached = prepared => reached.unwrap(),
            result = &mut operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
        }
    }

    let detail = fixture.host.request_drain.fail_stop_detail().unwrap();
    let retry = fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap_err();
    let lease = current(&fixture.host, &record);
    assert!(matches!(retry, WorkerRpcError::Transport(_)));
    assert!(retry.to_string().contains(&detail));
    assert_eq!(
        fixture.host.request_drain.fail_stop_detail().unwrap(),
        detail
    );
    assert!(detail.contains(POST_PREPARED_CANCEL_DETAIL));
    assert_eq!(lease.state(), SecondaryCustodyState::Preparing);
    assert!(lease.retains_platform());
    assert!(!lease.is_wire_ready());
    assert!(lease.monitor_is_pending());
    release.send(()).unwrap();
    server.await.unwrap();
}

fn current(host: &Host, record: &RuntimeRecord) -> Arc<SecondaryCustodyLease> {
    host.secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(record))
        .unwrap()
}
