use super::*;
use clap::Parser;
use yolo_runtime::cli::constants as cli;

const SESSION_ID: &str = "6ba7b811-9dad-11d1-80b4-00c04fd430c8";

#[test]
fn shell_only_create_cli_accepts_name_and_working_directory() {
    let valid = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        "/tmp/yolo",
        cli::CREATE_COMMAND,
        cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
        cli::NAME_ARGUMENT,
        "workspace",
        cli::WORKING_DIRECTORY_ARGUMENT,
        "/tmp/project",
    ]);
    assert!(valid.is_ok());
}

#[test]
fn operational_subcommands_parse() {
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::SERVE_COMMAND,
            cli::DETACHED_SESSION_ARGUMENT,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
        ])
        .is_ok()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::SESSION_WORKER_COMMAND,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
            cli::TERMINAL_ID_ARGUMENT,
            SESSION_ID,
        ])
        .is_ok()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::BRIDGE_COMMAND,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
        ])
        .is_ok()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
            cli::GET_COMMAND,
            SESSION_ID,
        ])
        .is_ok()
    );
}

#[test]
fn logging_is_limited_to_host_and_worker_commands() {
    let home = PathBuf::from("/tmp/yolo");
    let terminal_id = uuid::Uuid::parse_str(SESSION_ID).unwrap();
    let serve = Command::Serve {
        home: home.clone(),
        socket: None,
        detached_session: false,
        startup_guard: false,
    };
    assert!(command_emits_logs(&serve));
    assert!(command_emits_logs(&Command::SessionWorker {
        home,
        terminal_id,
    }));
    let bootstrap = Command::BootstrapChild;
    for command in [
        Command::BootstrapChild,
        Command::Bridge {
            stdio: true,
            home: PathBuf::new(),
            socket: None,
        },
    ] {
        assert!(!command_emits_logs(&command));
    }
    initialize_logging(&serve);
    initialize_logging(&bootstrap);
}

#[test]
fn attach_defaults_to_one_json_value() {
    let parsed = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        "/tmp/yolo",
        cli::ATTACH_COMMAND,
        SESSION_ID,
    ])
    .unwrap();
    let Command::Ctl(CtlOptions {
        command: CtlCommand::Attach { events, .. },
        ..
    }) = parsed.command
    else {
        panic!("attach command was not parsed");
    };
    assert_eq!(events, 0);
}

#[test]
fn control_existing_socket_mode_needs_no_home_and_rejects_missing_socket() {
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::SOCKET_ARGUMENT,
            "/tmp/broker.sock",
            cli::EXISTING_SOCKET_ARGUMENT,
            cli::LIST_COMMAND,
        ])
        .is_ok()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::EXISTING_SOCKET_ARGUMENT,
            cli::LIST_COMMAND,
        ])
        .is_err()
    );
}

#[test]
fn transaction_commands_require_creation_authorization_on_stdin() {
    let prefix = [
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        "/tmp/yolo",
    ];
    for command in [
        cli::CLEANUP_TERMINATE_COMMAND,
        cli::CLEANUP_ARCHIVE_COMMAND,
        cli::CLEANUP_DISCARD_COMMAND,
    ] {
        assert!(
            Cli::try_parse_from(prefix.into_iter().chain([
                command,
                SESSION_ID,
                cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
            ]))
            .is_ok()
        );
        assert!(Cli::try_parse_from(prefix.into_iter().chain([command, SESSION_ID])).is_err());
    }
    assert!(
        Cli::try_parse_from(prefix.into_iter().chain([
            cli::FINALIZE_CREATION_COMMAND,
            SESSION_ID,
            cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
        ]))
        .is_ok()
    );
    assert!(
        Cli::try_parse_from(
            prefix
                .into_iter()
                .chain([cli::FINALIZE_CREATION_COMMAND, SESSION_ID,])
        )
        .is_err()
    );
}

include!("tests_creation.rs");
include!("tests_creation_text.rs");
include!("../test_support/ctl_matrix.rs");
include!("tests_ctl_matrix.rs");
include!("tests_ctl_success.rs");
include!("tests_ctl_authorized_success.rs");
include!("tests_dispatch.rs");
include!("tests_session_initialization.rs");
include!("tests_existing_host.rs");
include!("tests_lifecycle.rs");
include!("tests_writer_authorization.rs");
