use super::signal_hook::run_pre_signal_hook;
use super::{process_identity_matches, process_start_time};
use anyhow::{Context, Result};

#[path = "inventory_member_signal.rs"]
mod member_signal;
pub use member_signal::signal_process_group_member;

const FORCE_SIGNAL_REQUIRES_TERMINATION_DETAIL: &str =
    "SIGKILL requires the typed force-termination operation";

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/inventory_contains.rs"]
mod contains_tests;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// Durable process locator used by persisted runtime records.
pub struct ProcessIdentity {
    pub pid: u32,
    pub start_time: u64,
}

impl ProcessIdentity {
    pub fn capture(pid: u32) -> Result<Self> {
        Ok(Self {
            pid,
            start_time: process_start_time(pid)?,
        })
    }

    pub fn matches(self) -> bool {
        process_identity_matches(self.pid, self.start_time)
    }

    pub fn observe(self) -> super::ProcessIdentityObservation {
        super::observe_process_identity(self.pid, self.start_time)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn try_capture(pid: u32) -> Result<Option<Self>> {
        #[cfg(target_os = "macos")]
        return Ok(super::macos_kernel::metadata(pid)?.map(|metadata| Self {
            pid: metadata.pid,
            start_time: metadata.start_time,
        }));
        #[cfg(not(target_os = "macos"))]
        match Self::capture(pid) {
            Ok(identity) => Ok(Some(identity)),
            Err(_) if !super::is_process_alive(pid) => Ok(None),
            Err(error) => Err(error),
        }
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn signal(self, signal: i32) -> Result<bool> {
        let Some(identity) = super::LiveProcessIdentity::capture(self)? else {
            return Ok(false);
        };
        identity.signal(signal)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessGroupIdentity {
    id: u32,
    members: Vec<super::LiveProcessIdentity>,
    authority: super::signal_authority::ForegroundSignalAuthority,
}

impl ProcessGroupIdentity {
    /// Read-only membership proof for an already captured foreground process group.
    pub(super) fn contains_current(&self, expected: ProcessIdentity) -> Result<bool> {
        let Some(current) = super::LiveProcessIdentity::capture(expected)? else {
            return Ok(false);
        };
        Ok(self.members.contains(&current)
            && current.try_matches()?
            && process_group(current.pid())? == Some(self.id))
    }

    pub(super) fn verified(id: u32, members: Vec<super::LiveProcessIdentity>) -> Self {
        Self {
            id,
            members,
            authority: super::signal_authority::ForegroundSignalAuthority::verified(),
        }
    }

    #[cfg(test)]
    pub(super) fn id(&self) -> u32 {
        self.id
    }
}

#[derive(Clone, Debug)]
pub struct PtyProcessScope {
    pub(super) root: super::LiveProcessIdentity,
    pub(super) durable_root: ProcessIdentity,
    pub(super) owned_session_id: Option<u32>,
}

#[cfg(not(target_os = "linux"))]
impl Copy for PtyProcessScope {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SessionOwnershipProof {
    LiveWitness,
    RootExitedUnreaped,
    RootGoneWithoutWait,
    SessionScanDisabled,
    SessionRetired,
}

impl SessionOwnershipProof {
    pub(super) fn root_is_gone(self) -> bool {
        matches!(self, Self::RootExitedUnreaped | Self::RootGoneWithoutWait)
    }

    pub(super) fn enables_capability_scan(self) -> bool {
        self.root_is_gone()
    }
}

pub fn signal_process_group(group: ProcessGroupIdentity, signal: i32) -> Result<bool> {
    #[cfg(not(test))]
    anyhow::ensure!(
        signal != libc::SIGKILL,
        FORCE_SIGNAL_REQUIRES_TERMINATION_DETAIL
    );
    run_pre_signal_hook();
    let mut delivered = false;
    let ProcessGroupIdentity {
        id,
        members,
        authority,
    } = group;
    for member in members {
        if member.try_matches()? && process_group(member.pid())? == Some(id) {
            delivered |= authority.signal(&member, signal)?;
        }
    }
    Ok(delivered)
}

pub(super) fn process_group(pid: u32) -> Result<Option<u32>> {
    let pid = i32::try_from(pid).context("process id exceeds platform range")?;
    let group = unsafe { libc::getpgid(pid) };
    if group >= 0 {
        return Ok(Some(group as u32));
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(None);
    }
    Err(error).context("inspect verified process group")
}
