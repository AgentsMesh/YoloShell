const CTL_AUTHORIZED_CREATE_NAME: &str = "authorized-create";

#[tokio::test]
async fn ctl_authorized_success_matrix_reaches_every_typed_stdin_command() {
    let commands = [
        vec![
            cli::CREATE_COMMAND,
            cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
            cli::NAME_ARGUMENT,
            CTL_AUTHORIZED_CREATE_NAME,
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
        run_authorized_ctl_success(&command).await;
    }
}

async fn run_authorized_ctl_success(command: &[&str]) {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join(CTL_MATRIX_SOCKET_NAME);
    let listener = bind_ctl_matrix_socket(&socket);
    let server = tokio::spawn(async move { serve_ctl_success_request(&listener).await });
    let mut options = parse_existing_socket_ctl(&socket, command);
    inject_ctl_authority(&mut options.command);

    let outcome = run_command(Command::Ctl(options)).await;

    tokio::time::timeout(CTL_MATRIX_SERVER_TIMEOUT, server)
        .await
        .unwrap()
        .unwrap();
    outcome.unwrap();
}

fn inject_ctl_authority(command: &mut CtlCommand) {
    match command {
        CtlCommand::Input { authorization, .. }
        | CtlCommand::Resize { authorization, .. }
        | CtlCommand::Signal { authorization, .. } => {
            authorization.set_test_input(yolo_capnp_codec::encode(&test_authorization()).unwrap());
        }
        CtlCommand::Create { authorization, .. } => authorization.set_test_input(
            yolo_capnp_codec::encode(&ctl_args::CreateAuthority {
                creation_token: uuid::Uuid::new_v4(),
            })
            .unwrap(),
        ),
        CtlCommand::CleanupTerminate { authorization, .. }
        | CtlCommand::CleanupArchive { authorization, .. }
        | CtlCommand::CleanupDiscard { authorization, .. } => authorization.set_test_input(
            yolo_capnp_codec::encode(&ctl_args::CleanupAuthority {
                creation_capability: uuid::Uuid::new_v4(),
            })
            .unwrap(),
        ),
        CtlCommand::FinalizeCreation { authorization, .. } => authorization.set_test_input(
            yolo_capnp_codec::encode(&ctl_args::FinalizeAuthority {
                creation_token: uuid::Uuid::new_v4(),
                creation_capability: uuid::Uuid::new_v4(),
            })
            .unwrap(),
        ),
        _ => panic!("authorized matrix received a command without typed stdin"),
    }
}

fn authorized_ctl_success_response(
    request: &yolo_runtime::protocol::RequestEnvelope,
) -> Option<yolo_runtime::protocol::ResponseEnvelope> {
    use yolo_runtime::protocol::{ResponseEnvelope, method};
    let response = match request.method.as_str() {
        method::CREATE_SESSION => ResponseEnvelope::success(
            request,
            &yolo_runtime::model::CreateTerminalResult {
                terminal_id: uuid::Uuid::new_v4(),
                creation_capability: uuid::Uuid::new_v4(),
            },
        ),
        method::TERMINAL_INPUT => {
            let input: yolo_runtime::worker::InputRequest = request.decode_payload().unwrap();
            ResponseEnvelope::success(
                request,
                &yolo_runtime::worker::InputResult {
                    input_id: input.input_id,
                    duplicate: false,
                    delivery: yolo_runtime::protocol::delivery::DELIVERED.to_owned(),
                },
            )
        }
        method::RESIZE_TERMINAL | method::SIGNAL_SESSION | method::FINALIZE_CREATION => {
            ResponseEnvelope::success(request, &())
        }
        method::CLEANUP_TERMINATE_SESSION => ResponseEnvelope::success(
            request,
            &yolo_runtime::model::CleanupTerminationOutcome::Terminated,
        ),
        method::CLEANUP_ARCHIVE_SESSION | method::CLEANUP_DISCARD_SESSION => {
            ResponseEnvelope::success(request, &ctl_matrix_session())
        }
        _ => return None,
    };
    Some(response.unwrap())
}
