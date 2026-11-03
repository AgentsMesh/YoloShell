use arguments::Arguments;
use clap::Parser;
use std::ffi::OsString;

pub struct TuiBuiltinClient;

impl yolo_builtin_client_host::BuiltinClientEntrypoint for TuiBuiltinClient {
    fn id(&self) -> &'static str {
        yolo_builtin_catalog::TUI_ID
    }
    fn run(&self, arguments: Vec<OsString>) -> anyhow::Result<()> {
        run_from(arguments)
    }
}

mod arguments;
mod audit;
mod command;
mod constants;
mod detach;
mod layouts;
mod lifecycle;
mod listing;
mod platform;
mod platform_events;
mod platform_label;
mod platform_snapshot;
mod recipe;
mod remote;
mod screen;
mod selection;
#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    let mut arguments = Arguments::try_parse_from(arguments)?;
    let command = arguments.command.take();
    let selection = arguments.selection()?;
    tokio::runtime::Runtime::new()?.block_on(selection.run(command))
}
