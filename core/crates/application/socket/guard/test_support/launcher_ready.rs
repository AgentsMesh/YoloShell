use super::super::constants::*;
use super::super::environment::validate_executable;
use super::super::protocol::read_new_capability;
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::io::BufRead;
use std::path::Path;
use std::process::Command;

#[cfg(target_os = "linux")]
pub(super) type Input = std::os::unix::net::UnixStream;
#[cfg(not(target_os = "linux"))]
pub(super) type Input = std::process::ChildStdin;

pub(super) fn prepare_launcher(
    reader: &mut impl BufRead,
    executable: &Path,
    socket_root: &Path,
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<Command> {
    validate_executable(executable)?;
    ensure!(socket_root.is_absolute(), LAUNCHER_CONTEXT);
    let _capability = read_new_capability(reader, FRAME_START).context(LAUNCHER_START_DETAIL)?;
    Ok(super::child_command(
        executable,
        socket_root,
        arguments,
        None,
    ))
}

#[cfg(target_os = "linux")]
pub(super) fn configure(command: &mut Command) -> Result<Input> {
    let (parent, child) = Input::pair()?;
    let child: std::os::fd::OwnedFd = child.into();
    command.stdin(std::process::Stdio::from(child));
    Ok(parent)
}

#[cfg(target_os = "linux")]
pub(super) fn wait(input: &mut Input) -> Result<()> {
    wait_for_ready(input, WATCHDOG_HANDSHAKE_TIMEOUT)
}

#[cfg(target_os = "linux")]
fn wait_for_ready(input: &mut Input, timeout: std::time::Duration) -> Result<()> {
    use std::io::Read;
    input.set_read_timeout(Some(timeout))?;
    let mut marker = [u8::default(); LAUNCHER_READY_MARKER.len()];
    input
        .read_exact(&mut marker)
        .context(LAUNCHER_READY_DETAIL)?;
    ensure!(marker == *LAUNCHER_READY_MARKER, LAUNCHER_READY_DETAIL);
    input.set_read_timeout(None)?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn announce() -> Result<()> {
    use std::io::Write;
    use std::os::fd::BorrowedFd;
    use yolo_process::{ProcessDiscoveryCapability, ProcessIdentity};
    // exec's CLOEXEC pipe closes before Linux publishes the new mm environment.
    // Reaching this user-space check proves the writer exists before watchdog tracking.
    let discovery = ProcessDiscoveryCapability::adopt_inherited_manager()?;
    discovery.verify_writer_holder(ProcessIdentity::capture(std::process::id())?)?;
    let inherited = unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) };
    let mut channel = Input::from(inherited.try_clone_to_owned()?);
    channel
        .write_all(LAUNCHER_READY_MARKER)
        .context(LAUNCHER_READY_DETAIL)?;
    Ok(())
}

#[cfg(all(test, target_os = "linux", feature = "yolo-unit-tests"))]
#[path = "../unit_tests/launcher_ready_tests.rs"]
mod tests;
