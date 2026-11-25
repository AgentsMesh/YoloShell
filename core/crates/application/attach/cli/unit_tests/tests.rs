use super::constants::{
    CONTROL_STATUS, REMOVED_STATUS_SOCKET_ARGUMENT, SESSION_ENDING_STATUS, SESSION_EXITED_STATUS,
    SESSION_RESUMED_STATUS, UNKNOWN_HOLDER_STATUS,
};
use super::status::status_message;
use super::*;
use clap::CommandFactory;
use yolo_relay::RelayStatus;
use yolo_runtime::cli::constants as cli;

const HOME_PATH: &str = "/tmp/yolo";
const SESSION_ID: &str = "f2f9aa0c-4b8a-48d1-b47c-bc670c0b37f0";
const INVALID_SESSION_ID: &str = "invalid";
const FIRST_HOLDER: &str = "holder-3";
const SECOND_HOLDER: &str = "holder-4";
const HOST_PATH: &str = "/tmp/yolo-host";
const STATUS_FD_VALUE: &str = "17";
const STATUS_FD: libc::c_int = 17;
const BROKER_SOCKET_PATH: &str = "/tmp/yolo-broker.sock";
const REMOVED_GENERATION_VALUE: &str = "9";

#[path = "runtime_tests.rs"]
mod runtime_tests;

#[test]
fn cli_requires_a_stable_holder_and_valid_uuid() {
    Cli::command().debug_assert();
    let valid = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::HOME_ARGUMENT,
        HOME_PATH,
        cli::TERMINAL_ID_ARGUMENT,
        SESSION_ID,
        cli::HOLDER_ID_ARGUMENT,
        FIRST_HOLDER,
    ])
    .unwrap();
    assert_eq!(valid.holder_id, FIRST_HOLDER);
    assert!(!valid.take_control);
    assert!(valid.status_fd.is_none());
    assert_eq!(valid.after_seq, INITIAL_OUTPUT_SEQUENCE);

    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::HOME_ARGUMENT,
            HOME_PATH,
            cli::TERMINAL_ID_ARGUMENT,
            SESSION_ID,
        ])
        .is_err()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::HOME_ARGUMENT,
            HOME_PATH,
            cli::TERMINAL_ID_ARGUMENT,
            INVALID_SESSION_ID,
            cli::HOLDER_ID_ARGUMENT,
            FIRST_HOLDER,
        ])
        .is_err()
    );
}

#[test]
fn cli_accepts_only_explicit_broker_mode_without_local_launch_inputs() {
    let remote = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::BROKER_SOCKET_ARGUMENT,
        BROKER_SOCKET_PATH,
        cli::TERMINAL_ID_ARGUMENT,
        SESSION_ID,
        cli::HOLDER_ID_ARGUMENT,
        FIRST_HOLDER,
    ])
    .unwrap();
    assert_eq!(
        remote.broker_socket,
        Some(PathBuf::from(BROKER_SOCKET_PATH))
    );
    assert!(remote.home.is_none());
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::BROKER_SOCKET_ARGUMENT,
            BROKER_SOCKET_PATH,
            cli::HOME_ARGUMENT,
            HOME_PATH,
            cli::TERMINAL_ID_ARGUMENT,
            SESSION_ID,
            cli::HOLDER_ID_ARGUMENT,
            FIRST_HOLDER,
        ])
        .is_err()
    );
}

#[test]
fn cli_supports_explicit_takeover_and_rejects_external_generation() {
    let parsed = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::HOME_ARGUMENT,
        HOME_PATH,
        cli::TERMINAL_ID_ARGUMENT,
        SESSION_ID,
        cli::HOLDER_ID_ARGUMENT,
        SECOND_HOLDER,
        cli::HOST_EXECUTABLE_ARGUMENT,
        HOST_PATH,
        cli::TAKE_CONTROL_ARGUMENT,
        cli::STATUS_FD_ARGUMENT,
        STATUS_FD_VALUE,
    ])
    .unwrap();
    assert!(parsed.take_control);
    assert_eq!(parsed.host_executable, Some(PathBuf::from(HOST_PATH)));
    assert_eq!(parsed.status_fd, Some(STATUS_FD));
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::HOME_ARGUMENT,
            HOME_PATH,
            cli::TERMINAL_ID_ARGUMENT,
            SESSION_ID,
            cli::HOLDER_ID_ARGUMENT,
            SECOND_HOLDER,
            cli::GENERATION_ARGUMENT,
            REMOVED_GENERATION_VALUE,
        ])
        .is_err()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::HOME_ARGUMENT,
            HOME_PATH,
            cli::TERMINAL_ID_ARGUMENT,
            SESSION_ID,
            cli::HOLDER_ID_ARGUMENT,
            SECOND_HOLDER,
            REMOVED_STATUS_SOCKET_ARGUMENT,
            HOME_PATH,
        ])
        .is_err()
    );
}

#[test]
fn status_messages_cover_control_read_only_recovery_and_unknown_holder() {
    assert_eq!(status_message(RelayStatus::ControlAcquired), CONTROL_STATUS);
    assert!(
        status_message(RelayStatus::ReadOnly { holder_id: None }).contains(UNKNOWN_HOLDER_STATUS)
    );
    assert!(
        status_message(RelayStatus::ControlLost {
            holder_id: Some(FIRST_HOLDER.into())
        })
        .contains(FIRST_HOLDER)
    );
    assert!(
        status_message(RelayStatus::HostStarted {
            executable: PathBuf::from(HOST_PATH)
        })
        .contains(HOST_PATH)
    );
    assert_eq!(
        status_message(RelayStatus::SessionResumed),
        SESSION_RESUMED_STATUS
    );
    assert_eq!(
        status_message(RelayStatus::SessionEnding),
        SESSION_ENDING_STATUS
    );
    assert_eq!(
        status_message(RelayStatus::SessionExited),
        SESSION_EXITED_STATUS
    );
}
