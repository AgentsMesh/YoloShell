use crate::worker::tests::*;
#[tokio::test]
async fn worker_connection_requires_valid_auth_and_serves_status() {
    let fixture = Fixture::new();
    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let unauthenticated = RequestEnvelope::new(
        "unauth",
        WORKER_STATUS_METHOD,
        yolo_capnp_value::Value::Null,
    );
    write_frame(&mut client, &unauthenticated).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::UNAUTHORIZED
    );
    task.await.unwrap().unwrap();

    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let bad_auth = RequestEnvelope::new(
        "bad-auth",
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: "wrong".into(),
            controller_generation: FIXTURE_CONTROLLER_GENERATION,
        })
        .unwrap(),
    );
    write_frame(&mut client, &bad_auth).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::UNAUTHORIZED
    );
    task.await.unwrap().unwrap();

    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let auth = RequestEnvelope::new(
        "auth",
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
    let mut client = WorkerConnection::open(client).await.unwrap();
    let status = RequestEnvelope::new(
        "status-request",
        WORKER_STATUS_METHOD,
        yolo_capnp_value::Value::Null,
    );
    let response = client.exchange(&status).await.unwrap();
    assert!(response.ok);
    drop(client);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn authenticated_attachment_replays_and_streams_output_events() {
    let fixture = Fixture::new();
    fixture
        .state
        .journal
        .lock()
        .unwrap()
        .append(b"history")
        .unwrap();
    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let auth = RequestEnvelope::new(
        "auth",
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: "test-token".into(),
            controller_generation: 1,
        })
        .unwrap(),
    );
    write_frame(&mut client, &auth).await.unwrap();
    let _: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    let mut client = WorkerConnection::open(client).await.unwrap();
    let attach = RequestEnvelope::new(
        "attach-request",
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq: 0,
        })
        .unwrap(),
    );
    let response = client.exchange(&attach).await.unwrap();
    let attached: AttachResult = response.decode_payload().unwrap();
    assert_eq!(attached.chunks[0].data, b"history");
    let live = OutputChunk {
        runtime_epoch: fixture.state.runtime_epoch(),
        sequence: 2,
        data: b"live".to_vec(),
    };
    fixture
        .state
        .events
        .send(WorkerEvent::Output(live.clone()))
        .unwrap();
    let message = client.next_event().await.unwrap();
    assert_eq!(message.event, event::TERMINAL_OUTPUT);
    let output: OutputChunk = from_value(message.payload).unwrap();
    assert_eq!(output.data, live.data);
    let exit = ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(0),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: 2,
    };
    fixture
        .state
        .events
        .send(WorkerEvent::Exited(exit))
        .unwrap();
    let message = client.next_event().await.unwrap();
    assert_eq!(message.event, event::SESSION_EXITED);
    drop(client);
    let _ = task.await.unwrap();
}
