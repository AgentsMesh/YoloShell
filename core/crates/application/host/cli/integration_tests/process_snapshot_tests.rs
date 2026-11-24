use super::*;
use clap::Parser;
use yolo_runtime::cli::constants as cli;

const ROOT_PID: &str = "123";
const ROOT_START_TIME: &str = "456";

#[test]
fn process_snapshot_subcommand_requires_an_exact_root_identity() {
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::PROCESS_SNAPSHOT_COMMAND,
            cli::ROOT_PID_ARGUMENT,
            ROOT_PID,
            cli::ROOT_START_TIME_ARGUMENT,
            ROOT_START_TIME,
            cli::PARENT_WATCH_ARGUMENT,
        ])
        .is_ok()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::PROCESS_SNAPSHOT_COMMAND,
            cli::ROOT_PID_ARGUMENT,
            ROOT_PID,
        ])
        .is_err()
    );
}

#[test]
fn process_snapshot_is_a_non_logging_command() {
    let command = Command::ProcessSnapshot {
        root_pid: ROOT_PID.parse().unwrap(),
        root_start_time: ROOT_START_TIME.parse().unwrap(),
        parent_watch: true,
        environment_keys: Vec::new(),
    };

    assert!(!command_emits_logs(&command));
}
