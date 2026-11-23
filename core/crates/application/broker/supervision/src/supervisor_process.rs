use anyhow::Result;
use std::ffi::OsString;
use std::process::{Command, Stdio};

pub(super) const SPAWN_CONTEXT: &str = "spawn supervised broker child";

pub(super) fn deadline(launch: &crate::ChildLaunch) -> Result<std::time::Instant> {
    launch
        .startup_deadline()
        .to_instant()
        .map(tokio::time::Instant::into_std)
}

pub(super) fn command(
    arguments: &[OsString],
    private_input: bool,
    authority: &str,
) -> Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(arguments)
        .env(
            yolo_cli::constants::BROKER_SUPERVISION_AUTHORITY_ENVIRONMENT,
            authority,
        )
        .stdin(if private_input {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Ok(command)
}
