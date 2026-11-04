use super::*;

const LIVE_WORKER_FIXTURE_COMMAND: &str = "/bin/sleep";
const LIVE_WORKER_FIXTURE_DURATION: &str = "30";
const RELEASE_ACK_FAILURE_DETAIL: &str =
    "Host cannot confirm Worker receipt after local PTY custody release";

#[tokio::test]
async fn release_ack_loss_waits_for_worker_exit_before_committing_release() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_release(
        support::ReleaseIdentity::Exact,
        ReleaseResponse::DropWithoutReading,
    );
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = super::current(&fixture.host, &record);
    let (mut worker, worker_identity) = live_worker();
    lease.probe_worker_identity(worker_identity);
    let mut states = lease.subscribe_state();

    fixture.close_worker_aliases();
    release.send(()).unwrap();
    server.await.unwrap();
    wait_for_state(&mut states, SecondaryCustodyState::ReleaseAckPending).await;
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    worker.kill().unwrap();
    worker.wait().unwrap();
    wait_for_state(&mut states, SecondaryCustodyState::Released).await;

    assert_eq!(worker_identity.observe(), ProcessIdentityObservation::Gone);
    assert!(!lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
}

#[tokio::test]
async fn release_ack_loss_with_current_worker_remains_fail_closed() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release) = fixture.serve_release(
        support::ReleaseIdentity::Exact,
        ReleaseResponse::DropWithoutReading,
    );
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = super::current(&fixture.host, &record);
    let (mut worker, worker_identity) = live_worker();
    lease.probe_worker_identity(worker_identity);
    let convergence_started = tokio::time::Instant::now();

    fixture.close_worker_aliases();
    release.send(()).unwrap();
    server.await.unwrap();
    super::wait_until(&fixture.host, || {
        fixture.host.request_drain.fail_stop_detail().is_some()
    })
    .await;

    assert!(
        convergence_started.elapsed()
            >= crate::session_timing::PROCESS_CAPABILITY_EXIT_EVENT_CONVERGENCE_TIMEOUT
    );
    assert_eq!(lease.state(), SecondaryCustodyState::ReleaseUncertain);
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(RELEASE_ACK_FAILURE_DETAIL)
    );
    worker.kill().unwrap();
    worker.wait().unwrap();
}

fn live_worker() -> (std::process::Child, ProcessIdentity) {
    let child = crate::process::spawn_waitable_test_child(
        std::process::Command::new(LIVE_WORKER_FIXTURE_COMMAND).arg(LIVE_WORKER_FIXTURE_DURATION),
    );
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    (child, identity)
}

async fn wait_for_state(
    states: &mut tokio::sync::watch::Receiver<SecondaryCustodyState>,
    expected: SecondaryCustodyState,
) {
    while *states.borrow_and_update() != expected {
        states.changed().await.unwrap();
    }
}
