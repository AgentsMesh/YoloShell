//! `yolo plugin` for trusted, user-level plugins.

mod arguments;
mod commands;
mod execution;
mod paths;

use std::ffi::OsString;

pub const COMMAND_INDEX: usize = 1;

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

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
