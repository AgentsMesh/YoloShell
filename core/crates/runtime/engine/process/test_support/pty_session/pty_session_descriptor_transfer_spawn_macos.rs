use super::*;
use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::os::fd::OwnedFd;
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub(super) fn helper_command(
    role: &str,
    ready: &Path,
    trigger: &Path,
    complete: &Path,
    socket: OwnedFd,
) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(HELPER_ROLE_ENV, role)
        .env(HELPER_READY_ENV, ready)
        .env(HELPER_TRIGGER_ENV, trigger)
        .env(HELPER_COMPLETE_ENV, complete)
        .stdin(Stdio::from(socket))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

pub(super) struct ChildGuard(Option<Child>);

impl ChildGuard {
    pub(super) fn spawn(command: Command) -> Self {
        Self(Some(crate::process::spawn_waitable_test_child(command)))
    }

    pub(super) fn id(&self) -> u32 {
        self.0.as_ref().unwrap().id()
    }

    pub(super) fn wait(&mut self) {
        self.0.take().unwrap().wait().unwrap();
    }
}

impl Drop for ChildGuard {
    #[cfg(test)]
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
