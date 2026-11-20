//! Read-only public protocol discovery. Never starts or attaches to a Hub.
mod arguments;
mod output;

use std::ffi::OsString;

pub const PROTOCOL_COMMAND: &str = "protocol";
pub const COMMAND_INDEX: usize = 1;
pub use arguments::ProtocolCommand;

pub fn selects(arguments: &[OsString]) -> bool {
    arguments
        .get(COMMAND_INDEX)
        .and_then(|value| value.to_str())
        == Some(PROTOCOL_COMMAND)
}

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    use clap::Parser;
    let arguments = arguments::Arguments::try_parse_from(arguments)?;
    let arguments::Command::Protocol { command } = arguments.command;
    run_command(command)
}

/// Runs one already parsed protocol command.
pub fn run_command(command: arguments::ProtocolCommand) -> anyhow::Result<()> {
    output::write_command(command, &mut std::io::stdout().lock())
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
