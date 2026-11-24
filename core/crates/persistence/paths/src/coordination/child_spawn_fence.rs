use crate::paths::private_file_lock::PrivateFileLock;
use anyhow::{Context, Result};
use std::os::fd::RawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;

const DESCRIPTOR_INSPECTION_CONTEXT: &str = "inspect coordination lock descriptor";
const STANDARD_DESCRIPTOR_ERROR: &str = "coordination lock occupies a standard descriptor";
const CLOSE_ON_EXEC_ERROR: &str = "coordination lock descriptor is not close-on-exec";
const FCNTL_FAILURE: libc::c_int = -1;
const CLOSE_SUCCESS: libc::c_int = 0;

#[derive(Clone, Copy)]
pub struct CoordinationChildSpawnFence {
    descriptor: RawFd,
}

impl CoordinationChildSpawnFence {
    pub(super) fn capture(lock: &PrivateFileLock) -> Result<Self> {
        let descriptor = lock.descriptor();
        validate_descriptor(descriptor)?;
        Ok(Self { descriptor })
    }

    pub fn configure(self, command: &mut Command) {
        // SAFETY: the closure captures one integer and calls only async-signal-safe close.
        unsafe {
            command.pre_exec(move || close_descriptor(self.descriptor));
        }
    }

    pub fn rearm_close_on_exec(self) -> Result<()> {
        let flags = unsafe { libc::fcntl(self.descriptor, libc::F_GETFD) };
        if flags == FCNTL_FAILURE {
            return Err(std::io::Error::last_os_error()).context(DESCRIPTOR_INSPECTION_CONTEXT);
        }
        if unsafe { libc::fcntl(self.descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) }
            == FCNTL_FAILURE
        {
            return Err(std::io::Error::last_os_error()).context(CLOSE_ON_EXEC_ERROR);
        }
        validate_descriptor(self.descriptor)
    }

    pub fn descriptor(self) -> RawFd {
        self.descriptor
    }
}

fn validate_descriptor(descriptor: RawFd) -> Result<()> {
    anyhow::ensure!(descriptor > libc::STDERR_FILENO, STANDARD_DESCRIPTOR_ERROR);
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags == FCNTL_FAILURE {
        return Err(std::io::Error::last_os_error()).context(DESCRIPTOR_INSPECTION_CONTEXT);
    }
    anyhow::ensure!(flags & libc::FD_CLOEXEC != 0, CLOSE_ON_EXEC_ERROR);
    Ok(())
}

fn close_descriptor(descriptor: RawFd) -> std::io::Result<()> {
    if unsafe { libc::close(descriptor) } == CLOSE_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/child_spawn_fence_tests.rs"]
mod tests;
