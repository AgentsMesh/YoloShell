mod arguments;
mod caller;
mod connection;
mod constants;
mod control;
mod control_commands;
mod conversation_text;
mod current;
mod labels;
mod lifecycle;
mod observe;
mod outcome;
mod queries;
mod receipt;
mod reject;
mod render;
mod runner;
mod runs;
mod wait;
mod wait_command;
mod wait_pages;

pub use arguments::{AgentCommand, Options};

pub use constants::COMMAND;

use clap::Parser;
use std::ffi::OsString;

const COMMAND_INDEX: usize = 1;

pub fn selects(arguments: &[OsString]) -> bool {
    arguments
        .get(COMMAND_INDEX)
        .and_then(|value| value.to_str())
        == Some(COMMAND)
}

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    let arguments = arguments::Arguments::try_parse_from(arguments)?;
    let arguments::Command::Agent(options) = arguments.command;
    run_options(options)
}

/// Runs one already parsed agent command with its connection selection.
pub fn run_options(options: arguments::Options) -> anyhow::Result<()> {
    run_arguments(arguments::Arguments {
        command: arguments::Command::Agent(options),
    })
}

fn run_arguments(arguments: arguments::Arguments) -> anyhow::Result<()> {
    let outcome = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(runner::run(arguments))?;
    outcome.print()?;
    if outcome.code != constants::EXIT_SUCCESS {
        std::process::exit(outcome.code);
    }
    Ok(())
}

#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
