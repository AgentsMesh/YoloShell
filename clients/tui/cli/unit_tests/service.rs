use super::constants::*;
use crate::arguments::Arguments;
use crate::command::Command;
use clap::Parser;

fn parse(command: &str, extra: &[&str]) -> Command {
    let mut arguments = vec![PROGRAM, command];
    arguments.extend_from_slice(extra);
    let arguments = Arguments::try_parse_from(arguments).unwrap();
    arguments.command.unwrap()
}

#[test]
fn lifecycle_commands_parse_without_connection_arguments() {
    assert!(matches!(
        parse(STATUS_COMMAND, &[]),
        Command::Status(options) if !options.json
    ));
    assert!(matches!(
        parse(VERSION_ENTRY, &[JSON_ARGUMENT]),
        Command::Version(options) if options.json
    ));
    assert!(matches!(parse(START_COMMAND, &[]), Command::Start(_)));
    assert!(matches!(parse(STOP_COMMAND, &[]), Command::Stop(_)));
    assert!(matches!(
        parse(RESTART_COMMAND, &[JSON_ARGUMENT]),
        Command::Restart(options) if options.json
    ));
}

#[test]
fn lifecycle_commands_help_names_the_service_scope() {
    let error = Arguments::try_parse_from([PROGRAM, STOP_COMMAND, HELP_ARGUMENT]).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("Stop the local Host and Hub"));
    assert!(text.contains("PTY"));
}
