const CTL_MATRIX_SOCKET_NAME: &str = "ctl.sock";
const CTL_MATRIX_REMOTE_ERROR: &str = "matrix request rejected";
const CTL_MATRIX_HOLDER: &str = "matrix-holder";
const CTL_MATRIX_GENERATION: &str = "1";
const CTL_MATRIX_COLUMNS: &str = "80";
const CTL_MATRIX_ROWS: &str = "24";
const CTL_MATRIX_SIGNAL: &str = "INT";
const CTL_MATRIX_INPUT_DATA: &str = "matrix-payload";
const CTL_MATRIX_CWD: &str = "/tmp";
const CTL_MATRIX_SOCKET_MODE: u32 = 0o600;
const CTL_MATRIX_SERVER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
#[tokio::test]
async fn ctl_network_command_matrix_reaches_each_rpc_branch() {
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
        run_ctl_remote_failure(&command).await;
    }
}

#[tokio::test]
async fn ctl_authorized_command_matrix_rejects_closed_stdin() {
    let commands = [
        vec![
            cli::CREATE_COMMAND,
            cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
            cli::WORKING_DIRECTORY_ARGUMENT,
            CTL_MATRIX_CWD,
        ],
        vec![
            cli::INPUT_COMMAND,
            SESSION_ID,
            cli::DATA_ARGUMENT,
            CTL_MATRIX_INPUT_DATA,
            cli::WRITER_AUTHORIZATION_STDIN_ARGUMENT,
        ],
        vec![
            cli::RESIZE_COMMAND,
            SESSION_ID,
            cli::COLUMNS_ARGUMENT,
            CTL_MATRIX_COLUMNS,
            cli::ROWS_ARGUMENT,
            CTL_MATRIX_ROWS,
            cli::WRITER_AUTHORIZATION_STDIN_ARGUMENT,
        ],
        vec![
            cli::SIGNAL_COMMAND,
            SESSION_ID,
            CTL_MATRIX_SIGNAL,
            cli::WRITER_AUTHORIZATION_STDIN_ARGUMENT,
        ],
        vec![
            cli::CLEANUP_TERMINATE_COMMAND,
            SESSION_ID,
            cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
        ],
        vec![
            cli::CLEANUP_ARCHIVE_COMMAND,
            SESSION_ID,
            cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
        ],
        vec![
            cli::CLEANUP_DISCARD_COMMAND,
            SESSION_ID,
            cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
        ],
        vec![
            cli::FINALIZE_CREATION_COMMAND,
            SESSION_ID,
            cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
        ],
    ];
    for command in commands {
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join(CTL_MATRIX_SOCKET_NAME);
        let _listener = bind_ctl_matrix_socket(&socket);
        assert!(run_ctl_matrix_command(&socket, &command).await.is_err());
    }
}

async fn run_ctl_remote_failure(command: &[&str]) {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join(CTL_MATRIX_SOCKET_NAME);
    let listener = bind_ctl_matrix_socket(&socket);
    yolo_remote_broker::validate_broker_socket(&socket).unwrap();
    let needs_runtime = matches!(command[0], cli::ACQUIRE_COMMAND | cli::TAKEOVER_COMMAND);
    let server = tokio::spawn(async move {
        if needs_runtime {
            let received = receive_ctl_matrix_request(&listener).await;
            assert_eq!(
                received.1.method,
                yolo_runtime::protocol::method::GET_SESSION
            );
            send_ctl_matrix_response(received, |request| {
                yolo_runtime::protocol::ResponseEnvelope::success(request, &ctl_matrix_session())
                    .unwrap()
            })
            .await;
        }
        let request = receive_ctl_matrix_request(&listener).await;
        send_ctl_matrix_response(request, |request| {
            yolo_runtime::protocol::ResponseEnvelope::failure(
                request,
                yolo_runtime::protocol::ErrorBody::not_found(CTL_MATRIX_REMOTE_ERROR),
            )
        })
        .await;
    });
    let outcome = run_ctl_matrix_command(&socket, command).await;
    assert!(outcome.is_err(), "{} unexpectedly succeeded", command[0]);
    tokio::time::timeout(CTL_MATRIX_SERVER_TIMEOUT, server)
        .await
        .unwrap_or_else(|_| panic!("{} did not connect: {outcome:?}", command[0]))
        .unwrap();
}

async fn receive_ctl_matrix_request(
    listener: &tokio::net::UnixListener,
) -> (
    yolo_runtime_rpc_testing::Peer,
    yolo_runtime::protocol::RequestEnvelope,
) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
    let request = stream.request().await.unwrap();
    (stream, request)
}

async fn send_ctl_matrix_response<F>(
    (mut stream, request): (
        yolo_runtime_rpc_testing::Peer,
        yolo_runtime::protocol::RequestEnvelope,
    ),
    response: F,
) where
    F: FnOnce(&yolo_runtime::protocol::RequestEnvelope) -> yolo_runtime::protocol::ResponseEnvelope,
{
    stream.respond(&response(&request)).await.unwrap();
    assert!(stream.request().await.is_err());
}

fn parse_existing_socket_ctl(socket: &std::path::Path, command: &[&str]) -> CtlOptions {
    let mut arguments = vec![
        yolo_launch_contract::EXECUTABLE_NAME.to_owned(),
        cli::CONTROL_COMMAND.to_owned(),
        cli::SOCKET_ARGUMENT.to_owned(),
        socket.to_string_lossy().into_owned(),
        cli::EXISTING_SOCKET_ARGUMENT.to_owned(),
    ];
    arguments.extend(command.iter().map(|value| (*value).to_owned()));
    let parsed = Cli::try_parse_from(arguments).unwrap();
    let Command::Ctl(options) = parsed.command else {
        panic!("control matrix command was not parsed");
    };
    options
}

fn bind_ctl_matrix_socket(path: &std::path::Path) -> tokio::net::UnixListener {
    use std::os::unix::fs::PermissionsExt;
    let listener = tokio::net::UnixListener::bind(path).unwrap();
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(CTL_MATRIX_SOCKET_MODE),
    )
    .unwrap();
    listener
}
