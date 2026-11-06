mod command;
mod constants;
mod install;
mod render;
mod status;

#[cfg(test)]
#[path = "../unit_tests/render.rs"]
mod render_tests;

pub use command::HookCommand;

use anyhow::Result;

pub fn run(command: HookCommand) -> Result<()> {
    match command {
        HookCommand::Status { json } => status::run(json),
        HookCommand::Install {
            providers,
            config_home,
            yes,
            dry_run,
            json,
        } => install::run(&providers, config_home.as_deref(), yes, dry_run, json),
    }
}
