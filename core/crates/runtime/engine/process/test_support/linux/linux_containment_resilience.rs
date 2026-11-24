use super::{LinuxContainmentBroker, LinuxContainmentLease, LinuxContainmentRecord};
use crate::process::{ProcessIdentity, PtySessionProof};
use anyhow::{Context, Result, ensure};
use std::os::fd::AsRawFd;

const DELEGATION_MISSING: &str = "resilience fixture has no delegated Linux cgroup root";
const CONTAINMENT_MISSING: &str = "resilience fixture containment has already disappeared";
const INCOMPLETE_PROOF: &str = "resilience fixture requires complete Linux PTY containment";
const THAW_VALUE: &str = "0\n";
const THAW_FAILED: &str = "resilience fixture cgroup rollback failed";
const FREEZE_ALREADY_OWNED: &str = "resilience fixture cannot take over an already frozen cgroup";

pub struct ResilienceContainmentRollback {
    leaf: std::fs::File,
    armed: bool,
    locked: bool,
}

impl ResilienceContainmentRollback {
    pub(in crate::process) fn capture(lease: &LinuxContainmentLease) -> Result<Self> {
        Ok(Self {
            leaf: lease.leaf.try_clone()?,
            armed: false,
            locked: false,
        })
    }

    pub fn begin_freeze(&mut self) -> Result<()> {
        ensure!(!self.locked, FREEZE_ALREADY_OWNED);
        let result = unsafe { libc::flock(self.leaf.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result != libc::c_int::default() {
            return Err(std::io::Error::last_os_error()).context(FREEZE_ALREADY_OWNED);
        }
        self.locked = true;
        ensure!(
            super::io::read_control(&self.leaf, super::constants::CGROUP_FREEZE_FILE)?.trim()
                == THAW_VALUE.trim(),
            FREEZE_ALREADY_OWNED
        );
        self.armed = true;
        Ok(())
    }

    pub fn commit_to_kill(&mut self) {
        self.armed = false;
    }

    pub fn is_frozen(&self) -> Result<bool> {
        const FROZEN_EVENT: &str = "frozen 1";
        Ok(
            super::io::read_control(&self.leaf, super::constants::CGROUP_EVENTS_FILE)?
                .lines()
                .any(|line| line == FROZEN_EVENT),
        )
    }
}

impl Drop for ResilienceContainmentRollback {
    fn drop(&mut self) {
        if self.armed {
            let result = super::io::write_control(
                &self.leaf,
                super::constants::CGROUP_FREEZE_FILE,
                THAW_VALUE,
            );
            if let Err(error) = result {
                match error {
                    super::LinuxContainmentError::Io(error)
                        if error.kind() == std::io::ErrorKind::NotFound => {}
                    error => eprintln!("{THAW_FAILED}: {error:#}"),
                }
            }
        }
        if self.locked {
            unsafe { libc::flock(self.leaf.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

pub(in crate::process) fn reopen(
    root: ProcessIdentity,
    record: LinuxContainmentRecord,
) -> Result<(LinuxContainmentLease, PtySessionProof)> {
    ensure!(
        record.validate_recovery(
            record.terminal_id,
            record.runtime_epoch,
            &crate::process::boot_id(),
            root,
        )?,
        INCOMPLETE_PROOF
    );
    let proof = PtySessionProof::restore_linux(
        record.pty_device.context(INCOMPLETE_PROOF)?,
        record.pty_capability.clone().context(INCOMPLETE_PROOF)?,
    )?;
    let broker = LinuxContainmentBroker::discover()?.context(DELEGATION_MISSING)?;
    let containment = broker
        .reopen_recovery(record)?
        .context(CONTAINMENT_MISSING)?;
    Ok((containment, proof))
}
