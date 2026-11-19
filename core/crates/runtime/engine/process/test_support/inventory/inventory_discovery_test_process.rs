use super::{TEST_SLEEP_DURATION, TEST_SLEEP_EXECUTABLE, TestProcess};
use crate::process::{MacosProcessCapabilityChildWriter, ProcessIdentity};
use std::process::{Command, Stdio};

impl TestProcess {
    pub(in crate::process) fn spawn_discovery_holder(
        environment: &str,
        capability: &str,
        writer: &MacosProcessCapabilityChildWriter,
    ) -> Self {
        let mut command = Command::new(TEST_SLEEP_EXECUTABLE);
        command
            .arg(TEST_SLEEP_DURATION)
            .env(environment, capability)
            .stderr(Stdio::null());
        writer.configure_exec_inheritance(&mut command);
        let child = crate::process::spawn_detached_with_control_pipes(command).unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self {
            child,
            identity,
            _directory: None,
        }
    }
}
