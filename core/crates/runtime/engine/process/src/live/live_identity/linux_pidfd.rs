use super::super::ProcessIdentity;
use crate::process::signal_authority::{ExactSignalOutcome, ProcessSignalAuthority};
use anyhow::{Context, Result};
use std::io;
use std::os::fd::OwnedFd;

#[path = "linux_pidfd/kernel.rs"]
mod kernel;
use kernel::KernelPidfdOperations;
#[path = "linux_pidfd/stop.rs"]
mod stop;

const OPEN_CONTEXT: &str = "open exact Linux process identity";
const POLL_CONTEXT: &str = "poll retained Linux process identity";
const SIGNAL_CONTEXT: &str = "signal retained Linux process identity";
const START_TIME_CONTEXT: &str = "validate retained Linux process identity";

pub(super) trait PidfdOperations {
    type Descriptor;

    fn open(&self, pid: u32) -> io::Result<Self::Descriptor>;
    fn start_time(&self, pid: u32) -> Result<Option<u64>>;
    fn exited(&self, descriptor: &Self::Descriptor) -> io::Result<bool>;
    fn send(&self, descriptor: &Self::Descriptor, signal: i32) -> io::Result<()>;
}

#[derive(Debug)]
pub(super) struct RetainedPidfd {
    descriptor: OwnedFd,
    identity: ProcessIdentity,
}

impl RetainedPidfd {
    pub(super) fn capture(identity: ProcessIdentity) -> Result<Option<Self>> {
        let Some(descriptor) = capture_with(&KernelPidfdOperations, identity)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            descriptor,
            identity,
        }))
    }

    pub(super) fn try_matches(&self, pid: u32, generation: u64) -> Result<bool> {
        if self.identity != identity(pid, generation) {
            return Ok(false);
        }
        KernelPidfdOperations
            .exited(&self.descriptor)
            .map(|exited| !exited)
            .context(POLL_CONTEXT)
    }

    pub(super) fn signal(
        &self,
        _authority: &ProcessSignalAuthority,
        pid: u32,
        generation: u64,
        signal: i32,
    ) -> Result<bool> {
        if self.identity != identity(pid, generation) {
            return Ok(false);
        }
        super::super::signal_hook::run_pre_signal_hook();
        signal_with(&KernelPidfdOperations, &self.descriptor, signal)
    }

    pub(super) fn signal_exact_sigkill(
        &self,
        _authority: &ProcessSignalAuthority,
        pid: u32,
        generation: u64,
    ) -> Result<ExactSignalOutcome> {
        if self.identity != identity(pid, generation) {
            return Ok(ExactSignalOutcome::IdentityReplaced);
        }
        super::super::signal_hook::run_pre_signal_hook();
        signal_exact_with(&KernelPidfdOperations, &self.descriptor, libc::SIGKILL)
    }

    pub(super) fn execution_state(
        &self,
        pid: u32,
        generation: u64,
    ) -> Result<super::ProcessExecutionState> {
        stop::execution_state(self, pid, generation)
    }
}

fn capture_with<O: PidfdOperations>(
    operations: &O,
    identity: ProcessIdentity,
) -> Result<Option<O::Descriptor>> {
    let descriptor = match operations.open(identity.pid) {
        Ok(descriptor) => descriptor,
        Err(error) if is_process_gone(&error) => return Ok(None),
        Err(error) => return Err(error).context(OPEN_CONTEXT),
    };
    let observed = operations
        .start_time(identity.pid)
        .context(START_TIME_CONTEXT)?;
    if observed != Some(identity.start_time) {
        return Ok(None);
    }
    if operations.exited(&descriptor).context(POLL_CONTEXT)? {
        return Ok(None);
    }
    Ok(Some(descriptor))
}

fn signal_with<O: PidfdOperations>(
    operations: &O,
    descriptor: &O::Descriptor,
    signal: i32,
) -> Result<bool> {
    signal_exact_with(operations, descriptor, signal)
        .map(|outcome| outcome == ExactSignalOutcome::Delivered)
}

fn signal_exact_with<O: PidfdOperations>(
    operations: &O,
    descriptor: &O::Descriptor,
    signal: i32,
) -> Result<ExactSignalOutcome> {
    match operations.send(descriptor, signal) {
        Ok(()) => Ok(ExactSignalOutcome::Delivered),
        Err(error) if is_process_gone(&error) => Ok(ExactSignalOutcome::ConfirmedAbsent),
        Err(error) => Err(error).context(SIGNAL_CONTEXT),
    }
}

fn identity(pid: u32, generation: u64) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        start_time: generation,
    }
}

pub(super) fn is_process_gone(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ESRCH)
}

pub(super) fn error_is_process_gone(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<io::Error>()
        .and_then(io::Error::raw_os_error)
        .is_some_and(|code| code == libc::ENOENT || code == libc::ESRCH)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/live_identity/linux_pidfd/kernel_tests.rs"]
mod kernel_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/linux_pidfd_tests.rs"]
mod tests;
