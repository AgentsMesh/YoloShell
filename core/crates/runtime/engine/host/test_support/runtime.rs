const HOST_TEST_AUTH_TOKEN: &str = "token";
const HOST_TEST_COMMAND: &str = "/bin/cat";
const HOST_TEST_RESPONSE_KIND: &str = "response";
const HOST_TEST_COLUMNS: u16 = 80;
const HOST_TEST_ROWS: u16 = 24;

fn host_runtime_record(host: &Host, terminal_id: Uuid, socket_path: PathBuf) -> RuntimeRecord {
    let record = RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: std::process::id(),
        worker_start_time: process::process_start_time(std::process::id()).unwrap_or_default(),
        child_pid: std::process::id(),
        child_start_time: process::process_start_time(std::process::id()).unwrap_or_default(),
        boot_id: host.boot_id.clone(),
        socket_path,
        auth_token: HOST_TEST_AUTH_TOKEN.into(),
        controller_generation: host.controller_generation,
        started_at_ms: now_ms(),
        command: HOST_TEST_COMMAND.into(),
        args: Vec::new(),
        cwd: host.paths.home().to_path_buf(),
        cols: HOST_TEST_COLUMNS,
        rows: HOST_TEST_ROWS,
        bootstrap_delivery: crate::model::BootstrapDelivery::NotRequired,
    };
    host.secondary_custody.authorize_rpc_test(&record);
    record
}

async fn accept_worker_auth(
    mut stream: UnixStream,
    record: &RuntimeRecord,
) -> yolo_runtime_rpc_testing::Peer {
    let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
    let response = ResponseEnvelope::success(&auth, &test_worker_hello(record)).unwrap();
    write_frame(&mut stream, &response).await.unwrap();
    yolo_runtime_rpc_testing::Peer::new(stream)
}

fn test_worker_hello(record: &RuntimeRecord) -> crate::model::WorkerHello {
    crate::model::WorkerHello {
        runtime: record.clone(),
        capabilities: Vec::new(),
    }
}

fn response_without_error(request: &RequestEnvelope) -> ResponseEnvelope {
    ResponseEnvelope {
        version: crate::protocol::PROTOCOL_VERSION,
        kind: HOST_TEST_RESPONSE_KIND.into(),
        request_id: request.request_id.clone(),
        method: request.method.clone(),
        ok: false,
        payload: yolo_capnp_value::Value::Null,
        error: None,
    }
}

fn persist_host_session(host: &Host, manifest: &TerminalManifest, record: &RuntimeRecord) {
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    host.catalog.lock().unwrap().insert(manifest).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), manifest).unwrap();
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), record).unwrap();
    host.secondary_custody.authorize_rpc_test(record);
}

async fn open_host_rpc(stream: UnixStream) -> yolo_capnp_rpc::Connection {
    yolo_capnp_rpc::Connection::open(
        stream,
        yolo_capnp_rpc::Protocol::Endpoint,
        tokio::time::Instant::now() + HOST_REQUEST_TIMEOUT,
    )
    .await
    .unwrap()
}

async fn host_rpc(
    client: &yolo_capnp_rpc::Connection,
    request: &RequestEnvelope,
) -> ResponseEnvelope {
    let exchange = client.request(
        yolo_capnp_rpc::Method::Call,
        yolo_capnp_codec::to_value(request).unwrap(),
        tokio::time::Instant::now() + HOST_REQUEST_TIMEOUT,
    );
    tokio::pin!(exchange);
    let wall_deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    loop {
        tokio::select! {
            result = &mut exchange => return yolo_capnp_codec::from_value(result.unwrap().value).unwrap(),
            _ = tokio::task::yield_now() => {
                assert!(std::time::Instant::now() < wall_deadline, "Host RPC scheduling timed out");
            },
        }
    }
}
