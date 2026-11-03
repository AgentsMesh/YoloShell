use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use yolo_runtime::paths::Paths;

#[path = "bridge.rs"]
mod bridge;
#[path = "catalog_stream.rs"]
mod catalog_stream;
#[path = "ctl.rs"]
mod ctl;
#[path = "ctl_args.rs"]
mod ctl_args;
#[path = "ctl_creation_authorization.rs"]
mod ctl_creation_authorization;
mod executor;
mod hook_installation;
mod host_namespace;
#[path = "host_start.rs"]
mod host_start;
#[path = "process_snapshot.rs"]
mod process_snapshot;
mod provider_hook;
use ctl_args::{CreationAuthorization, WriterAuthorization};
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
use yolo_application_test_fixtures::executables as test_executables;

#[derive(Debug, Parser)]
#[command(name = yolo_launch_contract::ProcessRole::Host.display_name(), version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

include!("command.rs");

include!("ctl_command.rs");

/// Runs the Host process role using argv supplied by the executable dispatcher.
/// The first argument is the display program name, never a role-selection alias.
pub fn run_from(arguments: impl IntoIterator<Item = std::ffi::OsString>) -> Result<()> {
    let command = Cli::try_parse_from(arguments)?.command;
    match command {
        Command::Hooks(options) => hook_installation::run(options),
        Command::ProviderHook { provider, home }
            if provider_hook::client(&provider) == Some(yolo_runtime::agent::HookClient::CodexNative) =>
        {
            yolo_runtime::codex_hooks::run_native(Paths::new(home))
        }
        Command::ProcessSnapshot {
            root_pid,
            root_start_time,
            parent_watch,
            environment_keys,
        } => process_snapshot::write(
            root_pid,
            root_start_time,
            parent_watch,
            environment_keys,
        ),
        command => {
            initialize_session(&command)?;
            executor::build(&command)?.block_on(run_command(command))
        }
    }
}

fn initialize_session(command: &Command) -> Result<()> {
    if needs_detached_session(command) {
        yolo_runtime::process::establish_detached_session()?;
    }
    Ok(())
}

fn needs_detached_session(command: &Command) -> bool {
    matches!(
        command,
        Command::Serve {
            detached_session: true,
            ..
        }
    )
}

async fn run_command(command: Command) -> Result<()> {
    initialize_logging(&command);
    match command {
        Command::Hooks(options) => hook_installation::run(options),
        Command::Serve {
            home,
            socket,
            detached_session: _,
            startup_guard,
        } => host_start::serve(home, socket, startup_guard).await,
        Command::Bridge {
            stdio: _,
            home,
            socket,
        } => bridge::run(home, socket).await,
        Command::CatalogStream(options) => catalog_stream::run(options).await,
        Command::SessionWorker { home, terminal_id } => {
            yolo_runtime::worker::run(Paths::new(home), terminal_id).await
        }
        Command::CustodyGuardian { home, terminal_id } => {
            yolo_runtime::custody_guardian::run(home, terminal_id).await
        }
        Command::BootstrapChild => yolo_runtime::worker::run_bootstrap_child().await,
        Command::ProviderHook { provider, home } => provider_hook::run(&provider, home).await,
        Command::ProcessSnapshot {
            root_pid,
            root_start_time,
            parent_watch,
            environment_keys,
        } => process_snapshot::write(
            root_pid,
            root_start_time,
            parent_watch,
            environment_keys,
        ),
        Command::Ctl(options) => ctl::run(options).await,
    }
}

fn initialize_logging(command: &Command) {
    if command_emits_logs(command) {
        let _ = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(std::io::stderr)
            .try_init();
    }
}

fn command_emits_logs(command: &Command) -> bool {
    matches!(
        command,
        Command::Serve { .. } | Command::SessionWorker { .. } | Command::CustodyGuardian { .. }
    )
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_hook_cli_tests.rs"]
mod claude_hook_cli_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/codex_hook_cli_tests.rs"]
mod codex_hook_cli_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/provider_hook_cli_tests.rs"]
mod provider_hook_cli_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/entry.rs"]
mod entry_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/process_snapshot_tests.rs"]
mod process_snapshot_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
