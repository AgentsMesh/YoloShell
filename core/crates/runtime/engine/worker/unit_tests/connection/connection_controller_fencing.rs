const FENCED_ATTACHMENT_REQUEST_ID: &str = "fenced-attachment";

#[tokio::test]
async fn attachment_revalidates_controller_after_waiting_for_snapshot_admission() {
    let fixture = Fixture::new();
    let (mut client, task) =
        authenticated_worker_connection(&fixture, CONNECTION_EDGE_CONTROLLER_GENERATION).await;
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .arm();
    let request = RequestEnvelope::new(
        FENCED_ATTACHMENT_REQUEST_ID,
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq: CONNECTION_EDGE_EMPTY_BARRIER,
        })
        .unwrap(),
    );
    let pending = begin_worker_request(client, request);
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .wait_until_reached()
        .await;

    fixture
        .state
        .adopt_controller(CONNECTION_EDGE_REPLACEMENT_GENERATION)
        .await
        .unwrap();
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .resume();

    let (client, response) = pending.await.unwrap();
    let response = response.unwrap();
    assert_eq!(response.error.unwrap().code, CONNECTION_EDGE_CONFLICT_CODE);
    drop(client);
    task.await.unwrap().unwrap();
}
