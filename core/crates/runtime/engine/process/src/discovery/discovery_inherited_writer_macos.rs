use super::ProcessDiscoveryInheritedWriter;
use crate::process::constants::{
    PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV, PROCESS_DISCOVERY_CAPABILITY_ENV,
    PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV,
};
use anyhow::{Result, ensure};
use std::os::fd::{FromRawFd, OwnedFd};
use std::process::Command;

const DESCRIPTOR_DETAIL: &str = "process discovery capability descriptor is invalid";

impl ProcessDiscoveryInheritedWriter {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn exec_inheriting(&mut self, command: &mut Command) -> anyhow::Error {
        self.writer.exec_inheriting(command)
    }

    #[cfg(feature = "adversarial-test-hooks")]
    pub fn descriptor(&self) -> libc::c_int {
        self.writer.descriptor()
    }
}

pub(super) fn manager_value() -> Result<String> {
    value(PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV)
}

pub(super) fn writer_value() -> Result<String> {
    value(PROCESS_DISCOVERY_CAPABILITY_ENV)
}

fn value(environment: &str) -> Result<String> {
    let value = std::env::var(environment)?;
    super::super::validate_value(&value)?;
    Ok(value)
}

pub(super) fn descriptor() -> Result<OwnedFd> {
    let raw = std::env::var(PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV)?.parse()?;
    ensure!(raw > libc::STDERR_FILENO, DESCRIPTOR_DETAIL);
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}
