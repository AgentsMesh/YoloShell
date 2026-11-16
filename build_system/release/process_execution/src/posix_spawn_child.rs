use crate::constants::{PROCESS_POLL_INTERVAL, PROCESS_SETTLEMENT_EPOCH_GRACE, PROCESS_TERM_GRACE};
use crate::process_group::{signal as signal_group, signal_process};
use crate::process_group_members::has_other_members;
use crate::process_wait;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use yolo_process::{
    ProcessDiscoveryCapability, ProcessDiscoveryCustody, ProcessDiscoveryWriterInheritance,
    process_discovery_force_grace,
};

const PID_RANGE_CONTEXT: &str = "spawned process id exceeds the custody range";
const MISSING_CUSTODY_DETAIL: &str = "spawned process discovery custody is missing";
const CUSTODY_SIGNAL_CONTEXT: &str = "signal spawned process custody";
const CUSTODY_SIGNAL_SEPARATOR: &str = "; ";

pub(crate) struct SpawnedProcess {
    pub(crate) resources: crate::process_resources::ProcessResources,
    pub pid: libc::pid_t,
    pub stdout: File,
    pub stderr: File,
    stdin: Option<File>,
    status: Option<std::process::ExitStatus>,
    custody: Option<ProcessDiscoveryCustody>,
    group_settled: bool,
    settled: bool,
    #[cfg(test)]
    drop_signals_enabled: bool,
    #[cfg(test)]
    drop_reap_grace: std::time::Duration,
}

impl SpawnedProcess {
    pub(crate) fn new(pid: libc::pid_t, stdout: File, stderr: File, stdin: Option<File>) -> Self {
        Self {
            resources: crate::process_resources::ProcessResources::default(),
            pid,
            stdout,
            stderr,
            stdin,
            status: None,
            custody: None,
            group_settled: false,
            settled: false,
            #[cfg(test)]
            drop_signals_enabled: true,
            #[cfg(test)]
            drop_reap_grace: crate::constants::PROCESS_DROP_REAP_GRACE,
        }
    }

    pub(super) fn install_custody(
        &mut self,
        capability: ProcessDiscoveryCapability,
        inheritance: ProcessDiscoveryWriterInheritance,
    ) -> Result<()> {
        let pid = u32::try_from(self.pid).context(PID_RANGE_CONTEXT)?;
        self.custody = Some(ProcessDiscoveryCustody::capture_suspended(
            pid,
            capability,
            inheritance,
        )?);
        Ok(())
    }

    pub fn refresh_custody(&mut self) -> Result<()> {
        self.custody
            .as_mut()
            .context(MISSING_CUSTODY_DETAIL)?
            .refresh_if_needed()
    }

    pub fn settle_discovery(&mut self) -> Result<()> {
        self.custody
            .as_mut()
            .context(MISSING_CUSTODY_DETAIL)?
            .settle(
                PROCESS_TERM_GRACE,
                process_discovery_force_grace(
                    PROCESS_SETTLEMENT_EPOCH_GRACE,
                    PROCESS_POLL_INTERVAL,
                ),
                PROCESS_POLL_INTERVAL,
            )
    }

    pub fn exited(&self) -> Result<bool> {
        process_wait::exited(self.pid)
    }

    pub fn wait(&mut self) -> Result<std::process::ExitStatus> {
        loop {
            if let Some(status) = self.status {
                return Ok(status);
            }
            let mut status = libc::c_int::default();
            let result = unsafe { libc::waitpid(self.pid, &mut status, libc::c_int::default()) };
            if result == self.pid {
                use std::os::unix::process::ExitStatusExt;
                let status = std::process::ExitStatus::from_raw(status);
                self.status = Some(status);
                return Ok(status);
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                return Err(error).context("reap spawned process");
            }
        }
    }

    pub fn write_input(&mut self, input: Option<&[u8]>) -> Result<()> {
        if let Some(bytes) = input {
            self.stdin
                .as_mut()
                .context("capture spawned process stdin")?
                .write_all(bytes)?;
        }
        self.stdin = None;
        Ok(())
    }

    pub fn mark_group_settled(&mut self) {
        self.group_settled = true;
    }

    pub fn signal_custody(&mut self, signal: libc::c_int) -> Result<()> {
        let mut failures = Vec::new();
        let direct_running = if self.status.is_none() {
            match self.exited() {
                Ok(exited) => !exited,
                Err(error) => {
                    failures.push(format!("{error:#}"));
                    true
                }
            }
        } else {
            false
        };
        let group_has_members = if self.group_settled {
            Some(false)
        } else {
            match has_other_members(self.pid, self.pid) {
                Ok(has_members) => Some(has_members),
                Err(error) => {
                    failures.push(format!("{error:#}"));
                    None
                }
            }
        };
        if !self.group_settled
            && (direct_running || group_has_members.unwrap_or(self.status.is_none()))
        {
            if let Err(error) = signal_group(self.pid, signal) {
                failures.push(format!("{error:#}"));
            }
        } else if !self.group_settled && !direct_running && group_has_members == Some(false) {
            self.group_settled = true;
        }
        if direct_running && let Err(error) = signal_process(self.pid, signal) {
            failures.push(format!("{error:#}"));
        }
        anyhow::ensure!(
            failures.is_empty(),
            "{CUSTODY_SIGNAL_CONTEXT}: {}",
            failures.join(CUSTODY_SIGNAL_SEPARATOR)
        );
        Ok(())
    }

    pub fn settle(&mut self) {
        self.settled = true;
        self.resources.confirm_released();
    }
}

#[path = "posix_spawn_child_drop.rs"]
mod drop_support;
