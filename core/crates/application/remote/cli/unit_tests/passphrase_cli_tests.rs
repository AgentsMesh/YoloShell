use super::super::{Cli, Command};
use super::fixture;
use clap::Parser;
use yolo_runtime::cli::constants;

#[test]
fn parses_private_passphrase_stdin_without_accepting_a_secret_argument() {
    let cli = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        constants::START_COMMAND,
        constants::CONFIG_ARGUMENT,
        fixture::CONFIG_FILE,
        constants::STATE_DIRECTORY_ARGUMENT,
        fixture::STATE_DIRECTORY,
        constants::IDENTITY_PASSPHRASES_STDIN_ARGUMENT,
    ])
    .unwrap();
    let Command::Start(options) = cli.command else {
        panic!("wrong command")
    };
    assert!(options.identity_passphrases_stdin);
}
