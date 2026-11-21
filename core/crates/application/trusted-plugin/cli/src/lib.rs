//! `yolo plugin` for trusted, user-level plugins.

mod arguments;
mod commands;
mod constants;
mod execution;
mod install;
mod install_git;
mod install_lifecycle;
mod install_registry;
mod paths;

use std::ffi::OsString;

pub const COMMAND_INDEX: usize = 1;
pub use arguments::PluginCommand;

pub fn selects(arguments: &[OsString]) -> bool {
    arguments
        .get(COMMAND_INDEX)
        .and_then(|value| value.to_str())
        == Some("plugin")
}

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    use clap::Parser;
    let arguments = arguments::Arguments::try_parse_from(arguments)?;
    commands::run(arguments)
}

/// Runs one already parsed plugin command.
pub fn run_plugin(command: arguments::PluginCommand) -> anyhow::Result<()> {
    commands::run_command(command)
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
