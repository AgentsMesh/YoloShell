use super::constants::*;
use super::socket_directory_io::{rename_exclusive, unlink};
use super::socket_identity::{SocketIdentity, bound_socket_identity_at, socket_identity_at};
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result, ensure};
use std::ffi::{CStr, CString};
use std::os::fd::RawFd;
use uuid::Uuid;

#[path = "socket_directory_publish.rs"]
mod publish;

#[derive(Debug)]
pub(super) struct SocketDirectory {
    directory: PrivateDirectory,
}

impl SocketDirectory {
    pub(super) fn new(directory: PrivateDirectory) -> Self {
        Self { directory }
    }

    pub(super) fn try_clone(&self) -> Result<Self> {
        self.directory.try_clone().map(Self::new)
    }

    pub(super) fn descriptor(&self) -> RawFd {
        self.directory.descriptor()
    }

    pub(super) fn path(&self) -> &std::path::Path {
        self.directory.path()
    }

    pub(super) fn verify_path(&self) -> Result<()> {
        self.directory.verify_path()
    }

    pub(super) fn identity(&self, name: &CStr) -> Result<Option<SocketIdentity>> {
        socket_identity_at(self.descriptor(), name).context(BROKER_SOCKET_INSPECT_RETAINED_CONTEXT)
    }

    pub(super) fn set_permissions(&self, name: &CStr, expected: SocketIdentity) -> Result<()> {
        ensure!(
            bound_socket_identity_at(self.descriptor(), name)? == Some(expected),
            BROKER_SOCKET_CHANGED
        );
        let mode = libc::mode_t::try_from(PRIVATE_FILE_MODE)?;
        let result = unsafe { libc::fchmodat(self.descriptor(), name.as_ptr(), mode, 0) };
        if result != 0 {
            return Err(std::io::Error::last_os_error()).context("set broker socket permissions");
        }
        ensure!(
            self.identity(name)? == Some(expected),
            BROKER_SOCKET_CHANGED
        );
        Ok(())
    }

    pub(super) fn remove_expected(&self, name: &CStr, expected: SocketIdentity) -> Result<()> {
        self.remove_expected_with_hook(name, expected, || Ok(()))
    }

    fn remove_expected_with_hook<F>(
        &self,
        name: &CStr,
        expected: SocketIdentity,
        after_quarantine: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let mut next_name = quarantine_name;
        self.remove_expected_using(name, expected, after_quarantine, &mut next_name)
    }

    fn remove_expected_using<F>(
        &self,
        name: &CStr,
        expected: SocketIdentity,
        after_quarantine: F,
        next_name: &mut dyn FnMut() -> Result<CString>,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let mut after_quarantine = Some(after_quarantine);
        for _ in 0..BROKER_SOCKET_QUARANTINE_ATTEMPTS {
            let quarantine = next_name()?;
            if !rename_exclusive(self.descriptor(), name, self.descriptor(), &quarantine)? {
                continue;
            }
            if let Some(hook) = after_quarantine.take()
                && let Err(error) = hook()
            {
                let restore = restore_between(self, &quarantine, self, name);
                return Err(with_restore(
                    error,
                    restore,
                    BROKER_SOCKET_QUARANTINE_RESTORE_FAILED,
                ));
            }
            let identity = bound_socket_identity_at(self.descriptor(), &quarantine);
            if matches!(&identity, Ok(Some(actual)) if *actual == expected) {
                unlink(self.descriptor(), &quarantine)?;
                self.directory.sync()?;
                return Ok(());
            }
            let error = identity
                .map(|_| anyhow::anyhow!(BROKER_SOCKET_CHANGED))
                .unwrap_or_else(|error| anyhow::Error::from(error).context(BROKER_SOCKET_CHANGED));
            let restore = restore_between(self, &quarantine, self, name);
            return Err(with_restore(
                error,
                restore,
                BROKER_SOCKET_QUARANTINE_RESTORE_FAILED,
            ));
        }
        anyhow::bail!(BROKER_SOCKET_QUARANTINE_EXHAUSTED)
    }
}

fn quarantine_name() -> Result<CString> {
    CString::new(format!(
        "{BROKER_SOCKET_QUARANTINE_PREFIX}{}",
        Uuid::new_v4()
    ))
    .context(BROKER_SOCKET_QUARANTINE_NAME_CONTEXT)
}

fn restore_between(
    source: &SocketDirectory,
    source_name: &CStr,
    target: &SocketDirectory,
    target_name: &CStr,
) -> Result<()> {
    ensure!(
        rename_exclusive(
            source.descriptor(),
            source_name,
            target.descriptor(),
            target_name,
        )?,
        BROKER_SOCKET_CHANGED
    );
    source.directory.sync()?;
    target.directory.sync()
}

fn with_restore(error: anyhow::Error, restore: Result<()>, context: &str) -> anyhow::Error {
    match restore {
        Ok(()) => error,
        Err(restore) => anyhow::anyhow!("{error:#}; {context}: {restore:#}"),
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/socket_directory_failure_tests.rs"]
mod failure_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/socket_directory_instance_matrix.rs"]
mod instance_matrix;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/socket_directory_tests.rs"]
mod tests;
