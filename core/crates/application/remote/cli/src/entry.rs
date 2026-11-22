use crate::{cli::Cli, entry_session, run};
use anyhow::Result;
use clap::Parser;
use std::ffi::OsString;

/// Runs the remote broker role with argv supplied by the executable dispatcher.
pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> Result<()> {
    // Preserve the former Tokio main entry: build and enter the multi-thread
    // runtime before parsing, then initialize the process session before dispatch.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let cli = Cli::try_parse_from(arguments)?;
            entry_session::initialize(&cli.command)?;
            run(cli).await
        })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/entry.rs"]
mod tests;
