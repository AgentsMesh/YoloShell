use super::*;
use clap::Parser;
use yolo_runtime::cli::constants;

const TEST_CONFIG: &str = "/tmp/config";
const TEST_STATE: &str = "/tmp/state";
const TEST_SOCKET: &str = "/tmp/socket";
const TEST_AUTHORITY_DOCUMENT: &str = "/tmp/host.authority.json";
const TEST_AUTHORITY_LOCK: &str = "/tmp/host.authority.lock";
const TEST_MUTATION_ID: &str = "01234567-89ab-4def-8123-456789abcdef";

#[test]
fn parses_server_and_management_commands() {
    let serve = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::SERVE_COMMAND,
        constants::CONFIG_ARGUMENT,
        TEST_CONFIG,
        constants::STATE_DIRECTORY_ARGUMENT,
        TEST_STATE,
        constants::EXPECTED_REVISION_ARGUMENT,
        fixture::TEST_CONFIG_REVISION,
        constants::EXPECTED_ANCHOR_ARGUMENT,
        fixture::TEST_STATE_ANCHOR,
    ])
    .unwrap();
    let Command::Serve(options) = serve.command else {
        panic!("wrong command")
    };
    assert_eq!(
        options.expected_revision.as_deref(),
        Some(fixture::TEST_CONFIG_REVISION)
    );
    assert_eq!(
        options.expected_anchor.as_deref(),
        Some(fixture::TEST_STATE_ANCHOR)
    );

    let start = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::START_COMMAND,
        constants::CONFIG_ARGUMENT,
        TEST_CONFIG,
        constants::STATE_DIRECTORY_ARGUMENT,
        TEST_STATE,
        constants::AUTHORITY_DOCUMENT_ARGUMENT,
        TEST_AUTHORITY_DOCUMENT,
        constants::AUTHORITY_LOCK_ARGUMENT,
        TEST_AUTHORITY_LOCK,
        constants::EXPECTED_PROFILE_SHA256_ARGUMENT,
        fixture::TEST_PROFILE_AUTHORITY_SHA256,
    ])
    .unwrap();
    let Command::Start(start) = start.command else {
        panic!("wrong command")
    };
    assert_eq!(
        start.authority_document.as_deref(),
        Some(std::path::Path::new(TEST_AUTHORITY_DOCUMENT))
    );
    assert_eq!(
        start.authority_lock.as_deref(),
        Some(std::path::Path::new(TEST_AUTHORITY_LOCK))
    );

    let status = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::STATUS_COMMAND,
        constants::SOCKET_ARGUMENT,
        TEST_SOCKET,
    ])
    .unwrap();
    assert!(matches!(status.command, Command::Status { .. }));
    let bootstrap = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::BOOTSTRAP_COMMAND,
        constants::SOCKET_ARGUMENT,
        TEST_SOCKET,
    ])
    .unwrap();
    assert!(matches!(bootstrap.command, Command::Bootstrap { .. }));
    let retire = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::RETIRE_COMMAND,
        constants::STATE_DIRECTORY_ARGUMENT,
        TEST_STATE,
        constants::EXPECTED_HOST_ARGUMENT,
        fixture::TEST_HOST_ID,
        constants::AUTHORITY_DOCUMENT_ARGUMENT,
        TEST_AUTHORITY_DOCUMENT,
        constants::AUTHORITY_LOCK_ARGUMENT,
        TEST_AUTHORITY_LOCK,
        constants::EXPECTED_MUTATION_ID_ARGUMENT,
        TEST_MUTATION_ID,
    ])
    .unwrap();
    let Command::Retire(retire) = retire.command else {
        panic!("wrong command")
    };
    assert_eq!(
        retire.expected_mutation_id,
        TEST_MUTATION_ID.parse::<uuid::Uuid>().unwrap()
    );
    let working_directory = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::WORKING_DIRECTORY_COMMAND,
        constants::SOCKET_ARGUMENT,
        TEST_SOCKET,
    ])
    .unwrap();
    assert!(matches!(
        working_directory.command,
        Command::WorkingDirectory { .. }
    ));
}

#[test]
fn retirement_rejects_missing_host_authority_arguments() {
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            constants::RETIRE_COMMAND,
            constants::STATE_DIRECTORY_ARGUMENT,
            TEST_STATE,
            constants::EXPECTED_HOST_ARGUMENT,
            fixture::TEST_HOST_ID,
        ])
        .is_err()
    );
}
