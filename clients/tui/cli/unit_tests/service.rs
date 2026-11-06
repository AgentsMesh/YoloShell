use super::constants::*;
use crate::arguments::Arguments;
use crate::command::Command;
use crate::server::ServerCommand;
use clap::Parser;

fn parse(path: &[&str], extra: &[&str]) -> Command {
    let mut arguments = vec![PROGRAM];
    arguments.extend_from_slice(path);
    arguments.extend_from_slice(extra);
    Arguments::try_parse_from(arguments).unwrap().command.unwrap()
}

#[test]
fn lifecycle_commands_parse_without_connection_arguments() {
    assert!(matches!(
        parse(&["server", STATUS_COMMAND], &[]),
        Command::Server {
            command: ServerCommand::Status(options)
        } if !options.json
    ));
    assert!(matches!(
        parse(&["server", VERSION_ENTRY], &[JSON_ARGUMENT]),
        Command::Server {
            command: ServerCommand::Version(options)
        } if options.json
    ));
    assert!(matches!(
        parse(&["server", START_COMMAND], &[]),
        Command::Server {
            command: ServerCommand::Start(_)
        }
    ));
    assert!(matches!(
        parse(&["server", STOP_COMMAND], &[]),
        Command::Server {
            command: ServerCommand::Stop(_)
        }
    ));
    assert!(matches!(
        parse(&["server", RESTART_COMMAND], &[JSON_ARGUMENT]),
        Command::Server {
            command: ServerCommand::Restart(options)
        } if options.json
    ));
}

#[test]
fn lifecycle_commands_help_names_the_service_scope() {
    let error = Arguments::try_parse_from([PROGRAM, "server", STOP_COMMAND, HELP_ARGUMENT])
        .unwrap_err();
    let text = error.to_string();
    assert!(text.contains("Stop the local Host and Hub"));
    assert!(text.contains("PTY"));
}
