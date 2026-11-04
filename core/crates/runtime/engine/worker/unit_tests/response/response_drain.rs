use crate::worker::tests::*;
const RESPONSE_DRAIN_TEST_METHOD: &str = crate::protocol::method::TERMINATE_RUNTIME_EXACT;
const RESPONSE_DRAIN_REQUEST_ID: &str = "response-drain-terminate";
pub(super) const RESPONSE_DRAIN_AUTH_REQUEST_ID: &str = "response-drain-auth";
pub(super) const RESPONSE_DRAIN_TEST_TIMEOUT: Duration = Duration::from_secs(1);
const RESPONSE_DRAIN_RACE_ATTEMPTS: usize = 64;
const RESPONSE_DRAIN_RACE_PARTICIPANTS: usize = 2;

pub(super) async fn commit_worker_shutdown_for_response_drain(state: &Arc<WorkerState>) {
    let permit = state.admit_system_mutation().await;
    state.commit_system_shutdown(&permit);
}

#[tokio::test]
async fn response_drain_real_commit_closes_admission_at_its_linearization_point() {
    let fixture = Fixture::new();
    commit_worker_shutdown_for_response_drain(&fixture.state).await;

    assert!(fixture.state.response_shutdown_is_committed());
    assert!(fixture.state.reserve_response_publication().is_err());
    assert!(fixture.state.wait_for_response_drain(Duration::ZERO).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn response_drain_commit_and_reservation_have_one_total_order() {
    for _ in 0..RESPONSE_DRAIN_RACE_ATTEMPTS {
        let fixture = Fixture::new();
        let barrier = Arc::new(tokio::sync::Barrier::new(RESPONSE_DRAIN_RACE_PARTICIPANTS));
        let reservation_state = Arc::clone(&fixture.state);
        let reservation_barrier = Arc::clone(&barrier);
        let reservation = tokio::spawn(async move {
            reservation_barrier.wait().await;
            reservation_state.reserve_response_publication()
        });
        let commit_state = Arc::clone(&fixture.state);
        let commit = tokio::spawn(async move {
            barrier.wait().await;
            commit_worker_shutdown_for_response_drain(&commit_state).await;
        });
        let reservation = reservation.await.unwrap();
        commit.await.unwrap();

        assert!(fixture.state.response_shutdown_is_committed());
        assert!(fixture.state.reserve_response_publication().is_err());
        drop(reservation);
        assert!(
            fixture
                .state
                .wait_for_response_drain(RESPONSE_DRAIN_TEST_TIMEOUT)
                .await
        );
    }
}

#[tokio::test]
async fn response_drain_shutdown_waits_for_pending_response_publication() {
    let fixture = Fixture::new();
    let ticket = fixture.state.reserve_response_publication().unwrap();
    let notified = fixture.state.shutdown.notified();
    tokio::pin!(notified);
    fixture.state.commit_response_shutdown();
    tokio::time::timeout(RESPONSE_DRAIN_TEST_TIMEOUT, &mut notified)
        .await
        .unwrap();
    assert!(!fixture.state.wait_for_response_drain(Duration::ZERO).await);
    ticket.publish();
    assert!(
        fixture
            .state
            .wait_for_response_drain(RESPONSE_DRAIN_TEST_TIMEOUT)
            .await
    );
}

#[tokio::test]
async fn response_drain_dropped_publication_releases_shutdown_once() {
    let fixture = Fixture::new();
    let ticket = fixture.state.reserve_response_publication().unwrap();
    let notified = fixture.state.shutdown.notified();
    tokio::pin!(notified);
    fixture.state.commit_response_shutdown();
    drop(ticket);
    tokio::time::timeout(RESPONSE_DRAIN_TEST_TIMEOUT, &mut notified)
        .await
        .unwrap();
    assert!(fixture.state.reserve_response_publication().is_err());
}

#[tokio::test]
async fn response_drain_includes_ordinary_requests_in_the_barrier() {
    let fixture = Fixture::new();
    let ticket = fixture.state.reserve_response_publication().unwrap();
    fixture.state.commit_response_shutdown();
    assert!(fixture.state.reserve_response_publication().is_err());
    drop(ticket);
}

#[tokio::test]
async fn response_drain_publishes_termination_before_shutdown_abort() {
    let fixture = Fixture::new();
    *lock_recover(&fixture.state.exit) = Some(ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(0),
        signal: None,
        reason: ExitReason::UserTerminate,
        observed_at_ms: now_ms(),
        last_output_sequence: 0,
    });
    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let auth = RequestEnvelope::new(
        RESPONSE_DRAIN_AUTH_REQUEST_ID,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: FIXTURE_AUTH_TOKEN.into(),
            controller_generation: FIXTURE_CONTROLLER_GENERATION,
        })
        .unwrap(),
    );
    write_frame(&mut client, &auth).await.unwrap();
    let _: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    let client = WorkerConnection::open(client).await.unwrap();

    fixture.state.control_test_hooks.response_before_write.arm();
    let mut shutdown = Box::pin(fixture.state.shutdown.notified());
    let terminate = RequestEnvelope::new(
        RESPONSE_DRAIN_REQUEST_ID,
        RESPONSE_DRAIN_TEST_METHOD,
        to_value(&FencedTerminateRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: fixture.state.runtime_epoch(),
            expected_status: None,
            force: false,
        })
        .unwrap(),
    );
    let pending = begin_worker_request(client, terminate);
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .wait_until_reached()
        .await;

    tokio::time::timeout(RESPONSE_DRAIN_TEST_TIMEOUT, &mut shutdown)
        .await
        .unwrap();
    assert!(!fixture.state.wait_for_response_drain(Duration::ZERO).await);
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .resume();
    let (client, response) = tokio::time::timeout(RESPONSE_DRAIN_TEST_TIMEOUT, pending)
        .await
        .unwrap()
        .unwrap();
    let response = response.unwrap();
    assert!(response.ok);
    assert!(
        fixture
            .state
            .wait_for_response_drain(RESPONSE_DRAIN_TEST_TIMEOUT)
            .await
    );
    drop(client);
    let _ = task.await;
}
