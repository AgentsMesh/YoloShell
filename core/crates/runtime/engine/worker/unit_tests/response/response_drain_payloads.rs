use crate::worker::tests::*;
const RESPONSE_DRAIN_PAYLOAD_TIMEOUT: Duration = Duration::from_secs(1);
const RESPONSE_DRAIN_EXACT_ID: &str = "response-drain-exact-terminate";
const RESPONSE_DRAIN_SHUTDOWN_ID: &str = "response-drain-worker-shutdown";

#[derive(Clone, Copy)]
enum ResponseDrainShutdownPayload {
    ExactTerminate,
    WorkerShutdown,
}

fn exited_response_drain_fixture() -> Fixture {
    let fixture = Fixture::new();
    *lock_recover(&fixture.state.exit) = Some(ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(0),
        signal: None,
        reason: ExitReason::UserTerminate,
        observed_at_ms: now_ms(),
        last_output_sequence: 0,
    });
    fixture
}

fn shutdown_request(fixture: &Fixture, payload: ResponseDrainShutdownPayload) -> RequestEnvelope {
    match payload {
        ResponseDrainShutdownPayload::ExactTerminate => RequestEnvelope::new(
            RESPONSE_DRAIN_EXACT_ID,
            method::TERMINATE_RUNTIME_EXACT,
            to_value(&FencedTerminateRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
                expected_status: Some(TerminalRuntimeStatus::Running),
                force: false,
            })
            .unwrap(),
        ),
        ResponseDrainShutdownPayload::WorkerShutdown => RequestEnvelope::new(
            RESPONSE_DRAIN_SHUTDOWN_ID,
            WORKER_SHUTDOWN_METHOD,
            to_value(&TerminalRef {
                terminal_id: fixture.terminal_id(),
            })
            .unwrap(),
        ),
    }
}

async fn wait_for_response_shutdown_commit(state: &WorkerState) {
    tokio::time::timeout(RESPONSE_DRAIN_PAYLOAD_TIMEOUT, async {
        while !state.response_shutdown_is_committed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

async fn assert_payload_response_precedes_shutdown(payload: ResponseDrainShutdownPayload) {
    let fixture = exited_response_drain_fixture();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, FIXTURE_CONTROLLER_GENERATION).await;
    fixture.state.control_test_hooks.response_before_write.arm();
    let mut shutdown = Box::pin(fixture.state.shutdown.notified());
    let pending = begin_worker_request(client, shutdown_request(&fixture, payload));
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .wait_until_reached()
        .await;

    wait_for_response_shutdown_commit(&fixture.state).await;
    tokio::time::timeout(RESPONSE_DRAIN_PAYLOAD_TIMEOUT, &mut shutdown)
        .await
        .unwrap();
    assert!(!fixture.state.wait_for_response_drain(Duration::ZERO).await);
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .resume();
    let (client, response) = pending.await.unwrap();
    let response = response.unwrap();
    assert!(response.ok);
    assert!(
        fixture
            .state
            .wait_for_response_drain(RESPONSE_DRAIN_PAYLOAD_TIMEOUT)
            .await
    );
    drop(client);
    let _ = task.await;
}

#[tokio::test]
async fn response_drain_covers_every_shutdown_payload() {
    for payload in [
        ResponseDrainShutdownPayload::ExactTerminate,
        ResponseDrainShutdownPayload::WorkerShutdown,
    ] {
        assert_payload_response_precedes_shutdown(payload).await;
    }
}
