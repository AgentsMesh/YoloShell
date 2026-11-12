use super::constants::{
    GUARD_MODE_ENVIRONMENT, LAUNCH_EXECUTABLE_ENVIRONMENT, LAUNCH_SOCKET_ROOT_ENVIRONMENT,
    LAUNCHER_MODE, WATCHDOG_MODE, WATCHDOG_PROTOCOL_DETAIL,
};
use anyhow::{Result, bail};
use std::ffi::OsStr;
use std::process::Command;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InvocationMode {
    Guard,
    Launcher,
    Watchdog,
}

pub(super) fn invocation_mode() -> Result<InvocationMode> {
    let Some(mode) = std::env::var_os(GUARD_MODE_ENVIRONMENT) else {
        return Ok(InvocationMode::Guard);
    };
    if mode == OsStr::new(LAUNCHER_MODE) {
        return Ok(InvocationMode::Launcher);
    }
    if mode == OsStr::new(WATCHDOG_MODE) {
        return Ok(InvocationMode::Watchdog);
    }
    bail!(WATCHDOG_PROTOCOL_DETAIL)
}

pub(super) fn remove_internal_environment(command: &mut Command) {
    command
        .env_remove(GUARD_MODE_ENVIRONMENT)
        .env_remove(LAUNCH_EXECUTABLE_ENVIRONMENT)
        .env_remove(LAUNCH_SOCKET_ROOT_ENVIRONMENT);
}
