use anyhow::{Context, Result, ensure};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const INVALID_SYSTEM_RESULT: libc::c_int = -1;

pub fn private_kqueue(context: &'static str) -> Result<OwnedFd> {
    let spawn_guard = super::waitable_child_spawn_guard()?;
    private_kqueue_in_spawn_boundary(context, &spawn_guard.boundary())
}

pub fn private_kqueue_in_spawn_boundary(
    context: &'static str,
    _boundary: &super::WaitableChildSpawnBoundary<'_>,
) -> Result<OwnedFd> {
    let raw = unsafe { libc::kqueue() };
    ensure!(raw != INVALID_SYSTEM_RESULT, context);
    let descriptor = unsafe { OwnedFd::from_raw_fd(raw) };
    super::macos_process_capability_fd::set_close_on_exec(descriptor.as_raw_fd(), true)
        .context(context)?;
    Ok(descriptor)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_private_descriptor_tests.rs"]
mod tests;
