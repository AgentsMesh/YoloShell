use super::*;

const LIVE_WORKER_FIXTURE_COMMAND: &str = "/bin/sleep";
const LIVE_WORKER_FIXTURE_DURATION: &str = "30";

#[tokio::test]
async fn worker_eof_waits_for_exact_exit_metadata_before_settling() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    let mut worker = crate::process::spawn_waitable_test_child(
        std::process::Command::new(LIVE_WORKER_FIXTURE_COMMAND).arg(LIVE_WORKER_FIXTURE_DURATION),
    );
    let worker_identity = ProcessIdentity::capture(worker.id()).unwrap();
    lease.probe_worker_identity(worker_identity);
    let mut states = lease.subscribe_state();

    disconnect.send(()).unwrap();
    server.await.unwrap();
    wait_for_state(&mut states, SecondaryCustodyState::Disconnected).await;
    tokio::time::sleep(crate::host::PROCESS_EXIT_POLL_INTERVAL).await;
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    worker.kill().unwrap();
    worker.wait().unwrap();
    fixture.close_worker_aliases();
    wait_until(&fixture.host, || {
        fixture
            .host
            .secondary_custody
            .current(&CustodyRuntimeIdentity::from_record(&record))
            .is_none()
    })
    .await;

    assert_eq!(worker_identity.observe(), ProcessIdentityObservation::Gone);
    assert_eq!(lease.state(), SecondaryCustodyState::Released);
    assert!(!lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
}

#[tokio::test]
async fn truncated_control_waits_for_exact_exit_metadata_before_settling() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, disconnect) = fixture.serve_truncated_control_holding();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = current(&fixture.host, &record);
    let mut worker = crate::process::spawn_waitable_test_child(
        std::process::Command::new(LIVE_WORKER_FIXTURE_COMMAND).arg(LIVE_WORKER_FIXTURE_DURATION),
    );
    let worker_identity = ProcessIdentity::capture(worker.id()).unwrap();
    lease.probe_worker_identity(worker_identity);
    let mut states = lease.subscribe_state();

    disconnect.send(()).unwrap();
    server.await.unwrap();
    wait_for_state(&mut states, SecondaryCustodyState::Disconnected).await;
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    worker.kill().unwrap();
    worker.wait().unwrap();
    fixture.close_worker_aliases();
    wait_until(&fixture.host, || {
        fixture
            .host
            .secondary_custody
            .current(&CustodyRuntimeIdentity::from_record(&record))
            .is_none()
    })
    .await;

    assert_eq!(worker_identity.observe(), ProcessIdentityObservation::Gone);
    assert_eq!(lease.state(), SecondaryCustodyState::Released);
    assert!(!lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
}

async fn wait_for_state(
    states: &mut tokio::sync::watch::Receiver<SecondaryCustodyState>,
    expected: SecondaryCustodyState,
) {
    while *states.borrow_and_update() != expected {
        states.changed().await.unwrap();
    }
}
