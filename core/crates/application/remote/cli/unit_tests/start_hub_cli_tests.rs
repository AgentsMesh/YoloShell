use crate::cli::{Cli, Command};
use crate::fixture_constants::INVALID_PERMIT_FD_ARGUMENT;
use clap::Parser;
use yolo_broker_launch_authority::{AUTHORITY_STDIO_ARGUMENT, START_HUB_COMMAND};
use yolo_launch_contract::EXECUTABLE_NAME;
use yolo_runtime::cli::constants::{
    AUTHORITY_DOCUMENT_ARGUMENT, CONFIG_ARGUMENT, STATE_DIRECTORY_ARGUMENT,
};

#[test]
fn hub_start_requires_authority_stdio_and_accepts_no_caller_selected_scope_or_fd() {
    let cli = Cli::try_parse_from([EXECUTABLE_NAME, START_HUB_COMMAND, AUTHORITY_STDIO_ARGUMENT])
        .unwrap();
    assert!(matches!(cli.command, Command::StartHub(options) if options.authority_stdio));
    assert!(Cli::try_parse_from([EXECUTABLE_NAME, START_HUB_COMMAND]).is_err());
    for argument in [
        INVALID_PERMIT_FD_ARGUMENT,
        CONFIG_ARGUMENT,
        STATE_DIRECTORY_ARGUMENT,
        AUTHORITY_DOCUMENT_ARGUMENT,
    ] {
        assert!(
            Cli::try_parse_from([
                EXECUTABLE_NAME,
                START_HUB_COMMAND,
                AUTHORITY_STDIO_ARGUMENT,
                argument,
                "other"
            ])
            .is_err()
        );
    }
}
