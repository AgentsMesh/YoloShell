//! Read-only conversation client commands. Uses existing Hub endpoints and never reads provider files.
mod arguments;
mod constants;
mod document;
mod runner;

use clap::Parser;
use std::{ffi::OsString, io::Write};

pub use arguments::{Action, Options};
pub use constants::COMMAND;
const COMMAND_INDEX: usize = 1;

pub fn selects(arguments: &[OsString]) -> bool {
    arguments
        .get(COMMAND_INDEX)
        .and_then(|value| value.to_str())
        == Some(COMMAND)
}

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    let arguments = arguments::Arguments::try_parse_from(arguments)?;
    let arguments::Command::Conversation(options) = arguments.command;
    run(options)
}

/// Runs one already parsed conversation command with its connection selection.
pub fn run(options: arguments::Options) -> anyhow::Result<()> {
    let request = document::request(&options.action, std::io::stdin().lock())?;
    let reply = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(runner::run(options, request))?;
    let mut output = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, &reply)?;
    writeln!(output)?;
    Ok(())
}

#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
