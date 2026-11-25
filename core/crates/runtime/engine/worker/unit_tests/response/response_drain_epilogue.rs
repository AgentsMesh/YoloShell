use crate::worker::tests::*;
const RESPONSE_DRAIN_EPILOGUE_ERROR: &str = "preserved runtime epilogue error";

fn pending_supervised_task(dropped: Arc<AtomicBool>) -> JoinSet<()> {
    let mut tasks = JoinSet::new();
    tasks.spawn(async move {
        let _probe = TaskDropProbe(dropped);
        std::future::pending::<()>().await;
    });
    tasks
}

async fn wait_for_epilogue_admission_close(state: &WorkerState) {
    while !state.response_shutdown_is_committed() {
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn response_drain_successful_epilogue_waits_before_task_abort() {
    let fixture = Fixture::new();
    let ticket = fixture.state.reserve_response_publication().unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let tasks = pending_supervised_task(Arc::clone(&dropped));
    tokio::task::yield_now().await;
    let state = Arc::clone(&fixture.state);
    let settlement = tokio::spawn(async move {
        settle_worker_runtime(&state, tasks, Ok(())).await
    });
    wait_for_epilogue_admission_close(&fixture.state).await;

    assert!(!dropped.load(Ordering::Acquire));
    ticket.publish();
    settlement.await.unwrap().unwrap();
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn response_drain_failing_epilogue_preserves_its_error() {
    let fixture = Fixture::new();
    let ticket = fixture.state.reserve_response_publication().unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let tasks = pending_supervised_task(Arc::clone(&dropped));
    tokio::task::yield_now().await;
    let state = Arc::clone(&fixture.state);
    let settlement = tokio::spawn(async move {
        settle_worker_runtime(
            &state,
            tasks,
            Err(anyhow::anyhow!(RESPONSE_DRAIN_EPILOGUE_ERROR)),
        )
        .await
    });
    wait_for_epilogue_admission_close(&fixture.state).await;

    assert!(!dropped.load(Ordering::Acquire));
    ticket.publish();
    let error = settlement.await.unwrap().unwrap_err();
    assert_eq!(error.to_string(), RESPONSE_DRAIN_EPILOGUE_ERROR);
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test(start_paused = true)]
async fn response_drain_timeout_cancels_connection_and_releases_ticket() {
    let fixture = Fixture::new();
    let (mut client, server) = UnixStream::pair().unwrap();
    let mut tasks = JoinSet::new();
    let state = Arc::clone(&fixture.state);
    tasks.spawn(async move { handle_connection(server, state).await });
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
    let authenticated: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert!(authenticated.ok);
    let deadline = tokio::time::Instant::now() + crate::session_timing::WORKER_READY_TIMEOUT
        + crate::session_timing::WORKER_READY_TIMEOUT;
    let client = yolo_capnp_rpc::Connection::open(client, yolo_capnp_rpc::Protocol::Endpoint, deadline).await.unwrap();

    fixture.state.control_test_hooks.response_before_write.arm();
    let pending = tokio::spawn(async move {
        client.request(yolo_capnp_rpc::Method::Call, yolo_capnp_codec::to_value(&status_request()).unwrap(), deadline).await
    });
    drive_rpc(fixture
        .state
        .control_test_hooks
        .response_before_write
        .wait_until_reached())
        .await;
    fixture.state.commit_response_shutdown();

    assert!(
        !fixture
            .state
            .wait_for_response_drain(crate::session_timing::WORKER_READY_TIMEOUT)
            .await
    );
    abort_and_drain_tasks(&mut tasks).await;
    assert!(
        drive_rpc(fixture
            .state
            .wait_for_response_drain(RESPONSE_DRAIN_CONNECTION_TIMEOUT))
            .await
    );
    pending.abort();
    let _ = pending.await;
}
