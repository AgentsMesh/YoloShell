use super::{waitable_child_spawn_guard, waitable_child_spawn_guard_before};
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

const DETACHED_SPAWN_CONTEXT: &str = "spawn detached process";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DetachedSessionOwner {
    Launcher,
    Child,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DetachedStandardError {
    Null,
    Inherit,
    Piped,
}

pub fn spawn_detached(
    executable: &Path,
    args: &[OsString],
    standard_error: DetachedStandardError,
) -> Result<Child> {
    spawn_detached_with_stdio(
        command_for(executable, args),
        standard_error,
        Stdio::null(),
        Stdio::null(),
        None,
    )
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub fn spawn_detached_with_control_pipes(command: Command) -> Result<Child> {
    spawn_detached_with_stdio(
        command,
        DetachedStandardError::Null,
        Stdio::piped(),
        Stdio::piped(),
        None,
    )
}

pub fn spawn_detached_before_deadline(
    executable: &Path,
    args: &[OsString],
    standard_error: DetachedStandardError,
    deadline: Instant,
) -> Result<Child> {
    super::command_private_input::require_budget(deadline)?;
    spawn_detached_with_stdio(
        command_for(executable, args),
        standard_error,
        Stdio::null(),
        Stdio::null(),
        Some(deadline),
    )
}

pub fn spawn_waitable_child_before_deadline(
    command: &mut Command,
    deadline: Instant,
) -> Result<Child> {
    super::command_private_input::require_budget(deadline)?;
    let _sigchld_guard = waitable_child_spawn_guard_before(deadline)?;
    super::command_private_input::require_budget(deadline)?;
    command.spawn().context("spawn waitable child process")
}

pub fn spawn_self_detaching(
    executable: &Path,
    args: &[OsString],
    standard_error: DetachedStandardError,
) -> Result<Child> {
    spawn_self_detaching_with_stdio(
        command_for(executable, args),
        standard_error,
        Stdio::null(),
        Stdio::null(),
        None,
    )
}

pub fn spawn_self_detaching_before_deadline(
    executable: &Path,
    args: &[OsString],
    standard_error: DetachedStandardError,
    deadline: Instant,
) -> Result<Child> {
    super::command_private_input::require_budget(deadline)?;
    spawn_self_detaching_with_stdio(
        command_for(executable, args),
        standard_error,
        Stdio::null(),
        Stdio::null(),
        Some(deadline),
    )
}

pub(super) fn command_for(executable: &Path, args: &[OsString]) -> Command {
    let mut command = Command::new(executable);
    command.args(args);
    command
}

pub(crate) fn spawn_detached_with_stdio(
    command: Command,
    standard_error: DetachedStandardError,
    standard_input: Stdio,
    standard_output: Stdio,
    deadline: Option<Instant>,
) -> Result<Child> {
    spawn_with_stdio(
        command,
        standard_error,
        standard_input,
        standard_output,
        deadline,
        DetachedSessionOwner::Launcher,
    )
}

pub(super) fn spawn_self_detaching_with_stdio(
    command: Command,
    standard_error: DetachedStandardError,
    standard_input: Stdio,
    standard_output: Stdio,
    deadline: Option<Instant>,
) -> Result<Child> {
    spawn_with_stdio(
        command,
        standard_error,
        standard_input,
        standard_output,
        deadline,
        DetachedSessionOwner::Child,
    )
}

pub(super) fn spawn_with_stdio(
    mut command: Command,
    standard_error: DetachedStandardError,
    standard_input: Stdio,
    standard_output: Stdio,
    deadline: Option<Instant>,
    session_owner: DetachedSessionOwner,
) -> Result<Child> {
    let standard_error = match standard_error {
        DetachedStandardError::Null => Stdio::null(),
        DetachedStandardError::Inherit => Stdio::inherit(),
        DetachedStandardError::Piped => Stdio::piped(),
    };
    command
        .stdin(standard_input)
        .stdout(standard_output)
        .stderr(standard_error);
    if session_owner == DetachedSessionOwner::Launcher {
        configure_detached_command(&mut command);
    }
    match deadline {
        Some(deadline) => spawn_waitable_child_before_deadline(&mut command, deadline),
        None => {
            let _sigchld_guard = waitable_child_spawn_guard()?;
            command.spawn().context(DETACHED_SPAWN_CONTEXT)
        }
    }
}

pub fn configure_detached_command(command: &mut Command) {
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}
