#[path = "../test_support/entry_constants.rs"]
mod constants;
use crate::cli::{Cli, Command};
use clap::{Parser, error::ErrorKind};
use constants::*;
use std::ffi::OsString;
use yolo_runtime::cli::constants::*;

#[test]
fn explicit_remote_argv_preserves_help_and_errors_without_changing_process_session() {
    let session = unsafe { libc::getsid(libc::pid_t::default()) };
    for (argument, expected) in [
        (HELP_ARGUMENT, ErrorKind::DisplayHelp),
        (VERSION_ARGUMENT, ErrorKind::DisplayVersion),
        (INVALID_ARGUMENT, ErrorKind::UnknownArgument),
    ] {
        let error = crate::run_from([PROGRAM_NAME, argument].map(OsString::from)).unwrap_err();
        let error = error.downcast::<clap::Error>().unwrap();
        assert_eq!(error.kind(), expected);
        if matches!(expected, ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
            assert!(error.to_string().contains(PROGRAM_NAME));
        }
        assert_eq!(unsafe { libc::getsid(libc::pid_t::default()) }, session);
    }
}

#[test]
fn remote_role_argv_accepts_hub_start_supervisor_and_child_commands() {
    let cli = Cli::try_parse_from([
        PROGRAM_NAME,
        yolo_broker_launch_authority::START_HUB_COMMAND,
        yolo_broker_launch_authority::AUTHORITY_STDIO_ARGUMENT,
    ])
    .unwrap();
    assert!(matches!(cli.command, Command::StartHub(_)));
    let supervisor = Cli::try_parse_from([
        PROGRAM_NAME,
        BROKER_SUPERVISOR_COMMAND,
        CONFIG_ARGUMENT,
        CONFIG,
        STATE_DIRECTORY_ARGUMENT,
        STATE,
        SUPERVISION_LAUNCHER_PID_ARGUMENT,
        LAUNCHER_PID,
        SUPERVISION_LAUNCHER_START_TIME_ARGUMENT,
        LAUNCHER_START,
    ])
    .unwrap();
    assert!(matches!(supervisor.command, Command::BrokerSupervisor(_)));
    let child = Cli::try_parse_from([
        PROGRAM_NAME,
        SUPERVISED_SERVE_COMMAND,
        CONFIG_ARGUMENT,
        CONFIG,
        STATE_DIRECTORY_ARGUMENT,
        STATE,
        SUPERVISION_ACTIVATION_FD_ARGUMENT,
        ACTIVATION_FD,
    ])
    .unwrap();
    assert!(matches!(child.command, Command::SupervisedServe(_)));
}

#[test]
fn malformed_supervisor_argv_is_rejected_before_detached_session_initialization() {
    let session = unsafe { libc::getsid(libc::pid_t::default()) };
    let error = crate::run_from([PROGRAM_NAME, BROKER_SUPERVISOR_COMMAND].map(OsString::from))
        .unwrap_err()
        .downcast::<clap::Error>()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    assert_eq!(unsafe { libc::getsid(libc::pid_t::default()) }, session);
}
