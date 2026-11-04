use super::*;

const REJECTED_COLUMNS: u16 = 91;
const REJECTED_ROWS: u16 = 27;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_unknown_and_stale_controller_requests_are_rejected() {
    let harness = Harness::start().await;
    let created = harness.create_cat("validation").await;
    let id = created.manifest.terminal_id;
    let target = runtime_ref_from_view(&created);
    let missing = harness
        .client()
        .get_session(Uuid::new_v4())
        .await
        .unwrap_err();
    assert!(matches!(missing, ClientError::Remote { code, .. } if code == error_code::NOT_FOUND));
    let unsupported = harness.client().raw("unknown_method", &()).await.unwrap();
    assert_eq!(
        unsupported.error.unwrap().code,
        error_code::UNSUPPORTED_METHOD
    );
    let lease = harness
        .client()
        .acquire_writer(target, "canonical-wire", None)
        .await
        .unwrap();
    let retired_generation = serde_json::json!({
        (field::TERMINAL_ID): id,
        (field::RUNTIME_EPOCH): target.runtime_epoch,
        (field::GENERATION): lease.generation,
        (field::LEASE_GENERATION): lease.generation,
        (field::COLS): REJECTED_COLUMNS,
        (field::ROWS): REJECTED_ROWS
    });
    let rejected = harness
        .client()
        .raw(method::RESIZE_TERMINAL, &retired_generation)
        .await
        .unwrap();
    let rejected = rejected.error.unwrap();
    assert_eq!(rejected.code, error_code::INVALID_REQUEST);
    let after: RuntimeRecord = read_json(&harness.paths().runtime_record(id)).unwrap();
    assert_eq!(
        (after.cols, after.rows),
        (created.manifest.cols, created.manifest.rows)
    );
    let invalid_create = harness
        .client()
        .create_session(&CreateTerminal {
            creation_token: Uuid::new_v4(),
            name: None,
            cwd: harness.home.path().to_path_buf(),
            cols: None,
            rows: None,
        })
        .await
        .unwrap();
    let invalid_view = harness
        .client()
        .get_session(invalid_create.terminal_id)
        .await
        .unwrap();
    harness.track_view(&invalid_view);
    harness
        .client()
        .terminate(invalid_create.terminal_id, true)
        .await
        .unwrap();

    let record: RuntimeRecord = read_json(&harness.paths().runtime_record(id)).unwrap();
    let mut worker_stream = tokio::net::UnixStream::connect(&record.socket_path)
        .await
        .unwrap();
    let stale_auth = RequestEnvelope::new(
        "stale-auth",
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: record.auth_token.clone(),
            controller_generation: 0,
        })
        .unwrap(),
    );
    write_frame(&mut worker_stream, &stale_auth).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut worker_stream).await.unwrap();
    assert_eq!(response.error.unwrap().code, error_code::CONFLICT);

    let mut unauthenticated = tokio::net::UnixStream::connect(&record.socket_path)
        .await
        .unwrap();
    let status = RequestEnvelope::new(
        "no-auth",
        WORKER_STATUS_METHOD,
        yolo_capnp_value::Value::Null,
    );
    write_frame(&mut unauthenticated, &status).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut unauthenticated).await.unwrap();
    assert_eq!(response.error.unwrap().code, error_code::UNAUTHORIZED);
}
