use super::command::{DetachedSessionOwner, DetachedStandardError, command_for, spawn_with_stdio};
use anyhow::{Context, Result};
use std::ffi::OsString;
#[cfg(test)]
use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
use std::time::Instant;

const PRIVATE_INPUT_CONTEXT: &str = "write detached process private input";
pub(super) const PRIVATE_INPUT_SETTLEMENT_CONTEXT: &str =
    "terminate detached process after private input failure";

pub fn spawn_detached_with_private_input(
    executable: &Path,
    args: &[OsString],
    standard_error: DetachedStandardError,
    private_input: &[u8],
    deadline: Instant,
) -> Result<Child> {
    spawn_with_private_input(
        command_for(executable, args),
        standard_error,
        private_input,
        deadline,
        DetachedSessionOwner::Launcher,
    )
}

pub fn spawn_self_detaching_with_private_input(
    executable: &Path,
    args: &[OsString],
    standard_error: DetachedStandardError,
    private_input: &[u8],
    deadline: Instant,
) -> Result<Child> {
    spawn_with_private_input(
        command_for(executable, args),
        standard_error,
        private_input,
        deadline,
        DetachedSessionOwner::Child,
    )
}

fn spawn_with_private_input(
    command: std::process::Command,
    standard_error: DetachedStandardError,
    private_input: &[u8],
    deadline: Instant,
    session_owner: DetachedSessionOwner,
) -> Result<Child> {
    super::command_private_input::require_budget(deadline)?;
    let reservation = super::child_reaper::prepare_detached_child_handoff()?;
    super::command_private_input::require_budget(deadline)?;
    let mut child = spawn_with_stdio(
        command,
        standard_error,
        Stdio::piped(),
        Stdio::null(),
        Some(deadline),
        session_owner,
    )?;
    let result = child
        .stdin
        .take()
        .context(PRIVATE_INPUT_CONTEXT)
        .and_then(|input| write_private_input_until(input, private_input, deadline));
    if let Err(error) = result {
        let settlement =
            super::command_settlement::terminate_after_private_input_failure(child, reservation);
        return Err(super::command_settlement::combine_private_input_failure(
            error, settlement,
        ));
    }
    Ok(child)
}

pub fn write_detached_child_private_input(
    input: ChildStdin,
    private_input: &[u8],
    deadline: Instant,
) -> Result<()> {
    write_private_input_until(input, private_input, deadline)
}

fn write_private_input_until(
    input: ChildStdin,
    private_input: &[u8],
    deadline: Instant,
) -> Result<()> {
    super::command_private_input::write_private_input_until(input, private_input, deadline)
}

#[cfg(test)]
pub(super) fn write_private_input(mut writer: impl Write, private_input: &[u8]) -> Result<()> {
    writer
        .write_all(private_input)
        .context(PRIVATE_INPUT_CONTEXT)
}
