mod check;
mod constants;
mod github;
mod github_release;
mod install;
mod report;
mod service;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::ffi::OsString;

#[derive(Debug, Parser)]
#[command(name = constants::PROGRAM, disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Update(check::Options),
}

pub fn selects(arguments: &[OsString]) -> bool {
    arguments
        .get(1)
        .is_some_and(|value| value == constants::COMMAND)
}

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> Result<()> {
    let cli = Cli::try_parse_from(arguments)?;
    match cli.command {
        Command::Update(options) => check::run(options),
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
