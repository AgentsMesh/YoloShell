use super::{
    FreezingProcessTree, ProcessClosureVersion, ProcessTransitionError, TrackedProducersGone,
};
use crate::process::linux_containment::LinuxContainmentPreparation;
use crate::process::{DescendantObservation, PtyMasterAliasReleaseAuthority};
use anyhow::{Context, Result};
use std::fs::File;
use std::os::fd::AsRawFd;

const ONE_DESCRIPTOR: libc::nfds_t = 1;
const NO_WAIT: libc::c_int = 0;
pub struct LinuxPtyHangup(ProcessClosureVersion);

impl FreezingProcessTree {
    /// An empty frozen cgroup and a hung-up master account for both contained
    /// processes and escaped slave holders when another login cannot be inspected.
    pub fn prepare_linux_hangup(
        &mut self,
        master: &File,
        binding: &PtyMasterAliasReleaseAuthority,
    ) -> Result<Option<LinuxPtyHangup>> {
        let tree = &mut self.tree;
        if !tree.root_exit.is_gone()
            || !tree.linux_containment_available()
            || tree.descendant_observation() != DescendantObservation::CapabilityScanPending
            || !tree.captured_targets().is_empty()
            || !tree.closure_version.binds_pty_output(binding)
        {
            return Ok(None);
        }
        let Some(members) = tree.destructive.poll_linux_frozen_members(false)? else {
            return Ok(None);
        };
        if !members.is_empty() {
            return Ok(None);
        }
        let Some(pty) = &tree.pty_proof else {
            return Ok(None);
        };
        if !pty.matches_linux_master(master)? {
            return Ok(None);
        }
        let removed = with_sealed_hangup(master, || {
            Ok(matches!(
                tree.destructive.prepare_linux_containment(&members)?,
                LinuxContainmentPreparation::Removed
            ))
        })?;
        Ok(removed.then(|| LinuxPtyHangup(tree.closure_version.clone())))
    }

    pub fn commit_linux_hangup(
        self,
        proof: LinuxPtyHangup,
    ) -> Result<TrackedProducersGone, ProcessTransitionError<Self>> {
        if self.tree.closure_version != proof.0
            || !self.tree.destructive.linux_containment_was_removed()
        {
            return Err(ProcessTransitionError::new(
                self,
                anyhow::anyhow!("PTY hangup belongs to another process closure"),
            ));
        }
        Ok(super::gone::complete(self.tree))
    }
}

fn with_sealed_hangup(master: &File, finish: impl FnOnce() -> Result<bool>) -> Result<bool> {
    if !drained_hangup(master)? {
        return Ok(false);
    }
    let mut previous = libc::c_int::default();
    ioctl(master, libc::TIOCGPTLCK, &mut previous)?;
    if previous == 0 {
        ioctl(master, libc::TIOCSPTLCK, &mut 1)?;
    }
    let result =
        drained_hangup(master).and_then(|closed| if closed { finish() } else { Ok(false) });
    if previous == 0 && !matches!(result, Ok(true)) {
        ioctl(master, libc::TIOCSPTLCK, &mut previous)?;
    }
    result
}

fn drained_hangup(master: &File) -> Result<bool> {
    let mut descriptor = libc::pollfd {
        fd: master.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        let ready = unsafe { libc::poll(&mut descriptor, ONE_DESCRIPTOR, NO_WAIT) };
        if ready >= 0 {
            return Ok(ready == 1 && descriptor.revents == libc::POLLHUP);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error).context("poll Linux PTY hangup");
        }
    }
}

fn ioctl(master: &File, request: libc::Ioctl, value: &mut libc::c_int) -> Result<()> {
    if unsafe { libc::ioctl(master.as_raw_fd(), request, value) } < 0 {
        return Err(std::io::Error::last_os_error()).context("inspect or set Linux PTY slave lock");
    }
    Ok(())
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/force/captured_tree_linux_hangup_tests.rs"]
mod tests;
