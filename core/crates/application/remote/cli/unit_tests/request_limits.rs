use super::*;
use clap::Parser;
use yolo_runtime::cli::constants;

#[test]
fn parses_command_and_file_limits() {
    let command = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::REMOTE_COMMAND,
        constants::SOCKET_ARGUMENT,
        fixture::MISSING_SOCKET_PATH,
        constants::PROGRAM_ARGUMENT,
        fixture::TEST_PROGRAM,
        constants::ARGUMENT_SEPARATOR,
        constants::STATUS_COMMAND,
    ])
    .unwrap();
    let Command::Exec(command) = command.command else {
        panic!("wrong command")
    };
    assert_eq!(command.args, [constants::STATUS_COMMAND]);
    let file = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::READ_FILE_COMMAND,
        constants::SOCKET_ARGUMENT,
        fixture::MISSING_SOCKET_PATH,
        constants::PATH_ARGUMENT,
        fixture::TEST_READ_PATH,
    ])
    .unwrap();
    let Command::ReadFile(file) = file.command else {
        panic!("wrong command")
    };
    assert_eq!(file.length, yolo_remote_broker::DEFAULT_FILE_READ_BYTES);
}
