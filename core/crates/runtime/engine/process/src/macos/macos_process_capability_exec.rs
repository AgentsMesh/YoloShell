use super::MacosProcessCapabilityChildWriter;
use anyhow::{Context, Error};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::Command;

const EXEC_INHERITANCE_GUARD_CONTEXT: &str = "acquire process capability exec inheritance gate";
const EXEC_INHERITANCE_ENABLE_CONTEXT: &str = "enable process capability exec inheritance";
const EXEC_INHERITANCE_RESTORE_CONTEXT: &str =
    "restore process capability close-on-exec after exec failure";
const EXEC_INHERITANCE_BACKUP_CONTEXT: &str = "back up process capability before exec";
const MINIMUM_BACKUP_DESCRIPTOR: libc::c_int =
    super::super::macos_process_capability_fd::MINIMUM_INHERITED_WRITER_DESCRIPTOR;
const FCNTL_FAILURE: libc::c_int = -1;
#[cfg(test)]
const INJECTED_RESTORE_ERROR: libc::c_int = libc::EIO;

impl MacosProcessCapabilityChildWriter {
    pub(crate) fn configure_exec_inheritance(&self, command: &mut Command) {
        let descriptor = self.descriptor.as_raw_fd();
        unsafe {
            command.pre_exec(move || super::set_close_on_exec(descriptor, false));
        }
    }

    pub fn exec_inheriting(&mut self, command: &mut Command) -> Error {
        let _spawn_guard = match super::super::waitable_child_spawn_guard()
            .context(EXEC_INHERITANCE_GUARD_CONTEXT)
        {
            Ok(guard) => guard,
            Err(error) => return error,
        };
        let descriptor = self.descriptor.as_raw_fd();
        let backup = match duplicate_cloexec(descriptor).context(EXEC_INHERITANCE_BACKUP_CONTEXT) {
            Ok(backup) => backup,
            Err(error) => return error,
        };
        if let Err(error) =
            super::set_close_on_exec(descriptor, false).context(EXEC_INHERITANCE_ENABLE_CONTEXT)
        {
            return error;
        }
        #[cfg(all(test, feature = "yolo-integration-tests"))]
        exec_test_hook::pause_after_inheritance_enabled();
        let exec_error = command.exec();
        match restore_close_on_exec(descriptor).context(EXEC_INHERITANCE_RESTORE_CONTEXT) {
            Ok(()) => Error::new(exec_error),
            Err(restore) => {
                #[cfg(all(test, feature = "yolo-integration-tests"))]
                let original_descriptor = descriptor;
                #[cfg(all(test, feature = "yolo-integration-tests"))]
                let backup_descriptor = backup.as_raw_fd();
                let original = std::mem::replace(&mut self.descriptor, backup);
                drop(original);
                #[cfg(all(test, feature = "yolo-integration-tests"))]
                exec_test_hook::pause_after_restore_fallback(
                    original_descriptor,
                    backup_descriptor,
                );
                restore.context(exec_error.to_string())
            }
        }
    }
}

fn restore_close_on_exec(descriptor: libc::c_int) -> std::io::Result<()> {
    #[cfg(all(test, feature = "yolo-integration-tests"))]
    if exec_test_hook::fail_next_restore() {
        return Err(std::io::Error::from_raw_os_error(INJECTED_RESTORE_ERROR));
    }
    super::set_close_on_exec(descriptor, true)
}

fn duplicate_cloexec(descriptor: libc::c_int) -> std::io::Result<OwnedFd> {
    let duplicated =
        unsafe { libc::fcntl(descriptor, libc::F_DUPFD_CLOEXEC, MINIMUM_BACKUP_DESCRIPTOR) };
    if duplicated == FCNTL_FAILURE {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(duplicated) })
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_exec_test_hook.rs"]
mod exec_test_hook;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_exec_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_shell_exec_tests.rs"]
mod shell_tests;
