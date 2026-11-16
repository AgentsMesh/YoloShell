//! `yolo workspace`: durable workspace metadata through the public Hub API.
//! Creating a workspace never creates a directory; the Host validates the root.

mod arguments;
mod commands;
mod connection;
mod constants;
mod members;
mod selector;
mod worktree;

use std::ffi::OsString;

use constants::COMMANDS;

const COMMAND_INDEX: usize = 1;

pub fn selects(arguments: &[OsString]) -> bool {
    arguments
        .get(COMMAND_INDEX)
        .and_then(|value| value.to_str())
        .is_some_and(|command| COMMANDS.contains(&command))
}

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    use clap::Parser;
    let arguments = arguments::Arguments::try_parse_from(arguments)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(commands::run(arguments))
}
