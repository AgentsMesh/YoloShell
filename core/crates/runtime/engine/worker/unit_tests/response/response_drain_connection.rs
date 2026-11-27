use crate::worker::tests::*;
const RESPONSE_DRAIN_STATUS_REQUEST_ID: &str = "response-drain-status";
pub(super) const RESPONSE_DRAIN_CONNECTION_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) fn status_request() -> RequestEnvelope {
    RequestEnvelope::new(
        RESPONSE_DRAIN_STATUS_REQUEST_ID,
        WORKER_STATUS_METHOD,
        yolo_capnp_value::Value::Null,
    )
}

#[tokio::test]
async fn response_drain_commit_before_admission_rejects_without_dispatch() {
    let fixture = Fixture::new();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, FIXTURE_CONTROLLER_GENERATION).await;
    fixture
        .state
        .control_test_hooks
        .response_before_admission
        .arm();
    let pending = begin_worker_request(client, status_request());
    fixture
        .state
        .control_test_hooks
        .response_before_admission
        .wait_until_reached()
        .await;

    commit_worker_shutdown_for_response_drain(&fixture.state).await;
    fixture
        .state
        .control_test_hooks
        .response_before_admission
        .resume();
    let (client, response) =
        tokio::time::timeout(RESPONSE_DRAIN_CONNECTION_TIMEOUT, pending)
            .await
            .unwrap()
            .unwrap();
    let response = response.unwrap();

    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::WORKER_SHUTTING_DOWN
    );
    assert!(fixture.state.reserve_response_publication().is_err());
    drop(client);
    let _ = task.await.unwrap();
}

#[tokio::test]
async fn response_drain_includes_an_admitted_ordinary_response() {
    let fixture = Fixture::new();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, FIXTURE_CONTROLLER_GENERATION).await;
    fixture.state.control_test_hooks.response_before_write.arm();
    let pending = begin_worker_request(client, status_request());
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .wait_until_reached()
        .await;

    fixture.state.commit_response_shutdown();
    let state = Arc::clone(&fixture.state);
    let mut drain = tokio::spawn(async move {
        state
            .wait_for_response_drain(RESPONSE_DRAIN_CONNECTION_TIMEOUT)
            .await
    });
    assert!(
        tokio::time::timeout(Duration::ZERO, &mut drain)
            .await
            .is_err()
    );

    fixture
        .state
        .control_test_hooks
        .response_before_write
        .resume();
    let (client, response) = pending.await.unwrap();
    let response = response.unwrap();
    assert!(response.ok);
    assert!(drain.await.unwrap());
    drop(client);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn response_drain_cancelled_connection_releases_its_ticket() {
    let fixture = Fixture::new();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, FIXTURE_CONTROLLER_GENERATION).await;
    fixture.state.control_test_hooks.response_before_write.arm();
    let pending = begin_worker_request(client, status_request());
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .wait_until_reached()
        .await;
    fixture.state.commit_response_shutdown();

    let state = Arc::clone(&fixture.state);
    let mut drain = tokio::spawn(async move {
        state
            .wait_for_response_drain(RESPONSE_DRAIN_CONNECTION_TIMEOUT)
            .await
    });
    assert!(
        tokio::time::timeout(Duration::ZERO, &mut drain)
            .await
            .is_err()
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(drain.await.unwrap());
    pending.abort();
    let _ = pending.await;
}

#[tokio::test]
async fn response_drain_includes_the_authentication_response() {
    let fixture = Fixture::new();
    let (mut client, task) = raw_worker_connection(&fixture).await;
    fixture.state.control_test_hooks.response_before_write.arm();
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
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .wait_until_reached()
        .await;

    commit_worker_shutdown_for_response_drain(&fixture.state).await;
    assert!(!fixture.state.wait_for_response_drain(Duration::ZERO).await);
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .resume();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert!(response.ok);
    assert!(
        fixture
            .state
            .wait_for_response_drain(RESPONSE_DRAIN_CONNECTION_TIMEOUT)
            .await
    );
    drop(client);
    task.await.unwrap().unwrap();
}
