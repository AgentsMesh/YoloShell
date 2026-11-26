use crate::cli::constants as cli;
use crate::process::ProcessIdentity;
use anyhow::{Context, Result};
use std::os::fd::OwnedFd;
use std::path::Path;
use std::process::{Command, Stdio};

const CLONE_CHILD_SOCKET_CONTEXT: &str = "clone isolated process scanner child socket";

pub(super) fn prepare(
    executable: &Path,
    root: ProcessIdentity,
    child_socket: std::os::unix::net::UnixStream,
    environment_keys: &[String],
) -> Result<Command> {
    let child_output = child_socket
        .try_clone()
        .context(CLONE_CHILD_SOCKET_CONTEXT)?;
    let child_input = OwnedFd::from(child_socket);
    let child_output = OwnedFd::from(child_output);
    let mut command = Command::new(executable);
    command.args(crate::cli::ProcessRole::Host.arguments());
    command
        .arg(cli::PROCESS_SNAPSHOT_COMMAND)
        .arg(cli::ROOT_PID_ARGUMENT)
        .arg(root.pid.to_string())
        .arg(cli::ROOT_START_TIME_ARGUMENT)
        .arg(root.start_time.to_string())
        .arg(cli::PARENT_WATCH_ARGUMENT);
    for key in environment_keys {
        command.arg(cli::ENVIRONMENT_KEY_ARGUMENT).arg(key);
    }
    command
        .stdin(Stdio::from(child_input))
        .stdout(Stdio::from(child_output))
        .stderr(Stdio::null())
        .env_clear();
    Ok(command)
}
