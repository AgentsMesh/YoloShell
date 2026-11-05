use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

const PRIMARY_FAILURE: &str = "injected exact custody activation failure";

#[tokio::test]
async fn incomplete_activation_retry_returns_the_original_fail_stop_reason() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let server = fixture.serve_activation(support::ActivationFixture::Missing);
    let first = fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap_err();
    server.await.unwrap();
    let lease = fixture
        .host
        .secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(&record))
        .unwrap();
    let diagnostic = fixture.host.request_drain.fail_stop_detail().unwrap();
    let retry = fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap_err();

    assert!(matches!(first, WorkerRpcError::Transport(_)));
    assert!(matches!(retry, WorkerRpcError::Transport(_)));
    assert!(retry.to_string().contains(&diagnostic));
    assert_eq!(lease.state(), SecondaryCustodyState::Preparing);
    assert!(lease.retains_platform());
    assert!(Arc::ptr_eq(
        &lease,
        &fixture
            .host
            .secondary_custody
            .current(&lease.runtime)
            .unwrap()
    ));
}

#[tokio::test]
async fn fail_stopped_host_does_not_reuse_a_ready_lease() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_holding();
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

    fixture.host.request_drain.fail_stop(PRIMARY_FAILURE);
    let error = fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap_err();

    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert!(error.to_string().contains(PRIMARY_FAILURE));
    assert!(lease.is_ready());
    assert!(lease.retains_platform());
    disconnect.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn admission_waiter_rechecks_fail_stop_after_acquiring_the_gate() {
    use std::future::{Future, poll_fn};
    use std::task::Poll;

    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let gate = fixture.host.secondary_custody.lock(record.terminal_id).await;
    let mut waiting = Box::pin(fixture.host.ensure_secondary_custody(&record));
    assert!(poll_fn(|context| Poll::Ready(waiting.as_mut().poll(context).is_pending())).await);

    fixture.host.request_drain.fail_stop(PRIMARY_FAILURE);
    drop(gate);
    let error = waiting.await.unwrap_err();

    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert!(error.to_string().contains(PRIMARY_FAILURE));
    disconnect.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn fail_stop_guard_retains_the_registry_gate_through_failure_publication() {
    use std::future::{Future, poll_fn};
    use std::task::Poll;

    let directory = tempdir().unwrap();
    let host = Host::open(crate::host::HostConfig::new(directory.path())).unwrap();
    let session = Uuid::new_v4();
    let mut guard =
        super::super::exchange_guard::PostPreparedFailStop::new(host.request_drain.clone());
    guard.bind_gate(host.secondary_custody.lock(session).await);
    guard.arm();
    let mut waiting = Box::pin(host.secondary_custody.lock(session));
    assert!(poll_fn(|context| Poll::Ready(waiting.as_mut().poll(context).is_pending())).await);

    guard.record_failure(&WorkerRpcError::transport(PRIMARY_FAILURE));
    assert!(poll_fn(|context| Poll::Ready(waiting.as_mut().poll(context).is_pending())).await);
    drop(guard);

    let _entered = waiting.await;
    assert!(
        host.request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(PRIMARY_FAILURE)
    );
}
