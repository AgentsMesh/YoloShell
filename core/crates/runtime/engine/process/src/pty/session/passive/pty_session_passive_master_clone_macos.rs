use super::MacosPassivePtyMaster;
use anyhow::{Context, Result, ensure};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const MINIMUM_PRIVATE_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 1;
const DUPLICATE_FAILURE: libc::c_int = -1;
const DUPLICATE_CONTEXT: &str = "duplicate exact passive PTY master";
const DUPLICATE_CHANGED_DETAIL: &str = "duplicated passive PTY master changed exact object";
const REVALIDATE_CHANGED_DETAIL: &str = "passive PTY master changed exact object";

impl MacosPassivePtyMaster {
    pub(in crate::process) fn duplicate_self(&self) -> Result<Self> {
        self.revalidate_self()?;
        let raw = unsafe {
            libc::fcntl(
                self.descriptor.as_raw_fd(),
                libc::F_DUPFD_CLOEXEC,
                MINIMUM_PRIVATE_DESCRIPTOR,
            )
        };
        if raw == DUPLICATE_FAILURE {
            return Err(std::io::Error::last_os_error()).context(DUPLICATE_CONTEXT);
        }
        let duplicate = Self::from_transferred(unsafe { OwnedFd::from_raw_fd(raw) })?;
        ensure!(
            duplicate.path == self.path && duplicate.identity == self.identity,
            DUPLICATE_CHANGED_DETAIL
        );
        Ok(duplicate)
    }

    pub(in crate::process) fn revalidate_self(&self) -> Result<()> {
        let (path, identity) =
            super::super::macos_object_lease::inspect_descriptor(&self.descriptor)?;
        ensure!(
            path == self.path && identity == self.identity,
            REVALIDATE_CHANGED_DETAIL
        );
        Ok(())
    }
}
