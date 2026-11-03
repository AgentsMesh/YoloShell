const CTL_SUCCESS_SINGLE_REQUEST: usize = 1;
const CTL_SUCCESS_RUNTIME_REQUESTS: usize = 2;
const CTL_SUCCESS_EVENT_COUNT: usize = 1;
const CTL_SUCCESS_BOOT_ID: &str = "ctl-success-boot";
const CTL_SUCCESS_CONTROLLER_GENERATION: u64 = 23;
const CTL_SUCCESS_OUTPUT_SEQUENCE_INCREMENT: u64 = 1;

#[tokio::test]
async fn ctl_success_matrix_decodes_and_prints_every_public_result() {
    let commands = [
        vec![cli::STATUS_COMMAND],
        vec![cli::LIST_COMMAND, cli::REQUIRE_COMPLETE_ARGUMENT],
        vec![cli::GET_COMMAND, SESSION_ID],
        vec![cli::SNAPSHOT_COMMAND, SESSION_ID],
        vec![cli::ATTACH_COMMAND, SESSION_ID],
        vec![
            cli::ACQUIRE_COMMAND,
            SESSION_ID,
            cli::HOLDER_ARGUMENT,
            CTL_MATRIX_HOLDER,
        ],
        vec![
            cli::TAKEOVER_COMMAND,
            SESSION_ID,
            cli::HOLDER_ARGUMENT,
            CTL_MATRIX_HOLDER,
            cli::OBSERVED_GENERATION_ARGUMENT,
            CTL_MATRIX_GENERATION,
        ],
        vec![cli::TERMINATE_COMMAND, SESSION_ID, cli::FORCE_ARGUMENT],
        vec![cli::ARCHIVE_COMMAND, SESSION_ID],
        vec![cli::DISCARD_COMMAND, SESSION_ID, cli::FORCE_ARGUMENT],
        vec![cli::RESUME_COMMAND, SESSION_ID, cli::RECREATE_ARGUMENT],
        vec![cli::RECONCILE_COMMAND],
    ];
    for command in commands {
        run_ctl_success(&command).await;
    }
}

async fn run_ctl_success(command: &[&str]) {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join(CTL_MATRIX_SOCKET_NAME);
    let listener = bind_ctl_matrix_socket(&socket);
    yolo_remote_broker::validate_broker_socket(&socket).unwrap();
    let request_count = ctl_success_request_count(command);
    let server = tokio::spawn(async move {
        for _ in 0..request_count {
            serve_ctl_success_request(&listener).await;
        }
    });
    let mut options = parse_existing_socket_ctl(&socket, command);
    if let CtlCommand::Attach { events, .. } = &mut options.command {
        *events = CTL_SUCCESS_EVENT_COUNT;
    }
    let outcome = run_command(Command::Ctl(options)).await;
    tokio::time::timeout(CTL_MATRIX_SERVER_TIMEOUT, server)
        .await
        .unwrap_or_else(|_| panic!("{} did not finish: {outcome:?}", command[0]))
        .unwrap();
    outcome.unwrap();
}

fn ctl_success_request_count(command: &[&str]) -> usize {
    if matches!(
        command[0],
        cli::SNAPSHOT_COMMAND | cli::ACQUIRE_COMMAND | cli::TAKEOVER_COMMAND
    ) {
        CTL_SUCCESS_RUNTIME_REQUESTS
    } else {
        CTL_SUCCESS_SINGLE_REQUEST
    }
}

async fn serve_ctl_success_request(listener: &tokio::net::UnixListener) {
    use yolo_runtime::protocol::{event, method};
    let (mut stream, request) = receive_ctl_matrix_request(listener).await;
    let attachment = request.method == method::ATTACH_SESSION;
    let response = ctl_success_response(&request);
    stream.respond(&response).await.unwrap();
    if attachment {
        let event = yolo_runtime::protocol::EventEnvelope::new(
            event::SESSION_UPDATED,
            Some(SESSION_ID.into()),
            Some(ctl_success_runtime_epoch().to_string()),
            &(),
        )
        .unwrap();
        stream.event(&event).await.unwrap();
    }
    assert!(stream.request().await.is_err());
}

fn ctl_success_response(
    request: &yolo_runtime::protocol::RequestEnvelope,
) -> yolo_runtime::protocol::ResponseEnvelope {
    use yolo_runtime::protocol::{ResponseEnvelope, method};
    match request.method.as_str() {
        method::HOST_STATUS => ResponseEnvelope::success(request, &ctl_success_host()).unwrap(),
        method::LIST_SESSIONS => ResponseEnvelope::success(
            request,
            &yolo_runtime::host::ListSessionsResult {
                sessions: vec![ctl_matrix_session()],
                complete: true,
            },
        )
        .unwrap(),
        method::GET_SESSION
        | method::ARCHIVE_SESSION
        | method::DISCARD_SESSION
        | method::RESUME_SESSION_EXACT => {
            ResponseEnvelope::success(request, &ctl_matrix_session()).unwrap()
        }
        method::GET_SNAPSHOT => {
            ResponseEnvelope::success(request, &ctl_success_snapshot()).unwrap()
        }
        method::ATTACH_SESSION => {
            ResponseEnvelope::success(request, &ctl_success_attachment()).unwrap()
        }
        method::ACQUIRE_WRITER | method::TAKEOVER_WRITER => {
            ResponseEnvelope::success(request, &test_grant()).unwrap()
        }
        method::TERMINATE_SESSION | method::RELEASE_WRITER => {
            ResponseEnvelope::success(request, &()).unwrap()
        }
        method::RECONCILE => {
            ResponseEnvelope::success(request, &yolo_runtime::host::ReconcileResult::default())
                .unwrap()
        }
        unexpected => authorized_ctl_success_response(request)
            .unwrap_or_else(|| panic!("unexpected ctl success method {unexpected}")),
    }
}

fn ctl_success_host() -> yolo_runtime::model::HostStatus {
    yolo_runtime::model::HostStatus {
        protocol_version: yolo_runtime::model::PROTOCOL_VERSION,
        boot_id: CTL_SUCCESS_BOOT_ID.into(),
        host_pid: CTL_MATRIX_WORKER_PID,
        controller_generation: CTL_SUCCESS_CONTROLLER_GENERATION,
        home: std::path::PathBuf::from(CTL_MATRIX_CWD),
        capabilities: Vec::new(),
    }
}

fn ctl_success_runtime_epoch() -> uuid::Uuid {
    uuid::Uuid::from_u128(CTL_MATRIX_RUNTIME_EPOCH_VALUE)
}

fn ctl_success_snapshot() -> yolo_runtime::model::TerminalSnapshot {
    yolo_runtime::model::TerminalSnapshot {
        runtime_epoch: ctl_success_runtime_epoch(),
        barrier_sequence: yolo_relay::INITIAL_OUTPUT_SEQUENCE,
        cols: yolo_runtime::model::DEFAULT_COLS,
        rows: yolo_runtime::model::DEFAULT_ROWS,
        data: Vec::new(),
    }
}

fn ctl_success_attachment() -> yolo_runtime::worker::AttachResult {
    yolo_runtime::worker::AttachResult {
        runtime_epoch: ctl_success_runtime_epoch(),
        snapshot: None,
        chunks: Vec::new(),
        next_sequence: yolo_relay::INITIAL_OUTPUT_SEQUENCE + CTL_SUCCESS_OUTPUT_SEQUENCE_INCREMENT,
        resynchronized: false,
    }
}
