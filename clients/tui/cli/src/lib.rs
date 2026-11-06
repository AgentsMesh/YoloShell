use arguments::Arguments;
use clap::Parser;
use std::ffi::OsString;

pub struct TuiBuiltinClient;

impl yolo_builtin_client_host::BuiltinClientEntrypoint for TuiBuiltinClient {
    fn id(&self) -> &'static str {
        yolo_tui_descriptor::TUI_ID
    }
    fn run(&self, arguments: Vec<OsString>) -> anyhow::Result<()> {
        run_from(arguments)
    }
}

mod arguments;
mod audit;
mod aux;
mod client;
mod command;
mod constants;
mod detach;
mod layouts;
mod lifecycle;
mod listing;
mod recipe;
mod remote;
mod screen;
mod selection;
mod server;
mod terminal;
#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    let mut arguments = Arguments::try_parse_from(arguments)?;
    let command = match arguments.command.take() {
        Some(command) => match aux::split(command) {
            aux::RootCommand::Aux(command) => {
                return aux::run(arguments.connection_options(), command);
            }
            aux::RootCommand::Tui(command) => Some(command),
        },
        None => None,
    };
    let selection = arguments.selection()?;
    tokio::runtime::Runtime::new()?.block_on(selection.run(command))
}
