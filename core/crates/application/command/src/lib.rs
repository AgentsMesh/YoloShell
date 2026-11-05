mod connection;
mod constants;
mod dispatch;
mod tree;

use clap::Parser;
use constants::{COMMANDS, TERMINAL_OPTIONS, VALUE_OPTIONS};
use std::ffi::OsString;
use yolo_launch_contract::{Routing, command_index};

pub use tree::{Cli, Command};

pub fn selects(arguments: &[OsString]) -> bool {
    if arguments
        .get(1)
        .and_then(|argument| argument.to_str())
        .is_some_and(|argument| COMMANDS.contains(&argument))
    {
        return true;
    }
    command_index(
        arguments,
        &Routing {
            commands: COMMANDS,
            value_options: VALUE_OPTIONS,
            terminal_options: TERMINAL_OPTIONS,
        },
    )
    .is_some()
}

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    let cli = Cli::try_parse_from(arguments)?;
    dispatch::run(cli.connection, cli.command)
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
