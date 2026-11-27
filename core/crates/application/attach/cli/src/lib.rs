use anyhow::Result;
use clap::Parser;
use std::os::fd::RawFd;
use std::path::PathBuf;
use yolo_relay::{
    INITIAL_OUTPUT_SEQUENCE, RawTerminal, RelayCancellation, RelayOptions, RelayRuntime,
    relay_session_with_status_and_cancellation, terminal_size, watch_window_changes,
};
use yolo_runtime::paths::Paths;

#[path = "constants.rs"]
mod constants;
mod executor;
#[path = "status.rs"]
mod status;
#[path = "protocol.rs"]
mod status_protocol;
mod terminal_input;
use status::StatusReporter;

#[derive(Debug, Parser)]
#[command(name = yolo_launch_contract::ProcessRole::Attach.display_name(), version)]
struct Cli {
    #[arg(long, required_unless_present = constants::BROKER_SOCKET_FIELD)]
    home: Option<PathBuf>,
    #[arg(long, conflicts_with = constants::BROKER_SOCKET_FIELD)]
    socket: Option<PathBuf>,
    #[arg(long, conflicts_with_all = [constants::HOME_FIELD, constants::SOCKET_FIELD, constants::HOST_EXECUTABLE_FIELD])]
    broker_socket: Option<PathBuf>,
    #[arg(long)]
    terminal_id: uuid::Uuid,
    #[arg(long)]
    holder_id: String,
    #[arg(long, conflicts_with = constants::BROKER_SOCKET_FIELD)]
    host_executable: Option<PathBuf>,
    #[arg(long)]
    take_control: bool,
    #[arg(long)]
    status_fd: Option<RawFd>,
    #[arg(long, default_value_t = INITIAL_OUTPUT_SEQUENCE)]
    after_seq: u64,
}

/// Runs the terminal relay role with an explicit argv, including its program name.
pub fn run_from(arguments: impl IntoIterator<Item = std::ffi::OsString>) -> Result<()> {
    let cli = Cli::try_parse_from(arguments)?;
    executor::build()?.block_on(run(cli, libc::STDIN_FILENO, RelayCancellation::default()))
}

async fn run(cli: Cli, input_fd: libc::c_int, cancellation: RelayCancellation) -> Result<()> {
    let reporter = StatusReporter::from_inherited_fd(cli.status_fd)?;
    run_with_reporter(cli, input_fd, cancellation, reporter).await
}

async fn run_with_reporter(
    cli: Cli,
    input_fd: libc::c_int,
    cancellation: RelayCancellation,
    reporter: StatusReporter,
) -> Result<()> {
    let runtime = relay_runtime(&cli)?;
    let initial_size = terminal_size(input_fd)?;
    let resize_events = watch_window_changes(input_fd)?;
    let _raw_terminal = RawTerminal::enable(input_fd)?;
    let options = RelayOptions::new(cli.terminal_id, cli.holder_id)?
        .after_sequence(cli.after_seq)
        .initial_size(initial_size)
        .take_control(cli.take_control);
    let (status_sender, status_events) = tokio::sync::mpsc::unbounded_channel();
    let reporter_cancellation = cancellation.clone();
    let relay = relay_session_with_status_and_cancellation(
        runtime,
        options,
        terminal_input::TerminalInput::new(input_fd)?,
        tokio::io::stdout(),
        resize_events,
        status_sender,
        cancellation,
    );
    tokio::pin!(relay);
    let reporting = reporter.run(status_events);
    tokio::pin!(reporting);
    tokio::select! {
        relay_result = &mut relay => {
            reporting.await?;
            relay_result?;
        }
        report_result = &mut reporting => match report_result {
            Ok(()) => {
                relay.await?;
            }
            Err(error) => {
                reporter_cancellation.cancel();
                let _ = relay.await;
                return Err(error);
            }
        }
    }
    Ok(())
}

fn relay_runtime(cli: &Cli) -> Result<RelayRuntime> {
    if let Some(socket) = &cli.broker_socket {
        let client = yolo_remote_broker::existing_runtime_client(socket);
        return Ok(RelayRuntime::broker(client)?);
    }
    let home = cli.home.clone().expect("clap requires local home");
    let paths = Paths::new(&home);
    let socket = cli
        .socket
        .clone()
        .map(Ok)
        .unwrap_or_else(|| paths.host_socket())?;
    Ok(RelayRuntime::managed(
        home,
        socket,
        cli.host_executable.clone(),
    )?)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/remote_deadline_tests.rs"]
mod remote_deadline_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/entry.rs"]
mod entry_tests;
