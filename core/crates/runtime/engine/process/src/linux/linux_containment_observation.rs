use super::{FREEZE_TIMEOUT, LinuxContainmentError};
use crate::process::{LiveProcessIdentity, ProcessIdentity, process_start_time};
use std::collections::{BTreeSet, HashSet};
use std::time::Instant;

const MEMBER_IDENTITY_DETAIL: &str = "frozen cgroup member identity is indeterminate";
const MEMBER_REPLACED_DETAIL: &str = "frozen cgroup member generation changed while observed";
const MEMBERSHIP_TIMEOUT_DETAIL: &str =
    "frozen cgroup membership did not stabilize before its deadline";

pub(super) fn capture_members(
    pids: &BTreeSet<u32>,
) -> Result<Option<HashSet<LiveProcessIdentity>>, LinuxContainmentError> {
    let mut identities = HashSet::with_capacity(pids.len());
    for pid in pids {
        let Some(live) = capture_member(*pid, process_start_time, LiveProcessIdentity::capture)?
        else {
            return Ok(None);
        };
        identities.insert(live);
    }
    Ok(Some(identities))
}

fn capture_member(
    pid: u32,
    mut start_time: impl FnMut(u32) -> anyhow::Result<u64>,
    retain: impl FnOnce(ProcessIdentity) -> anyhow::Result<Option<LiveProcessIdentity>>,
) -> Result<Option<LiveProcessIdentity>, LinuxContainmentError> {
    let Some(start) = observed_start(start_time(pid))? else {
        return Ok(None);
    };
    let retained = retain(ProcessIdentity {
        pid,
        start_time: start,
    })
    .map_err(|_| LinuxContainmentError::Identity(MEMBER_IDENTITY_DETAIL))?;
    if retained.is_some() {
        return Ok(retained);
    }
    match observed_start(start_time(pid))? {
        Some(current) if current != start => {
            Err(LinuxContainmentError::Identity(MEMBER_REPLACED_DETAIL))
        }
        // SIGKILL is permitted even while frozen. A dead/zombie member may still
        // appear in cgroup.procs until its kernel membership is retired.
        _ => Ok(None),
    }
}

fn observed_start(result: anyhow::Result<u64>) -> Result<Option<u64>, LinuxContainmentError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error)
            if error.chain().any(|cause| {
                cause.downcast_ref::<std::io::Error>().is_some_and(|io| {
                    io.kind() == std::io::ErrorKind::NotFound
                        || io.raw_os_error() == Some(libc::ESRCH)
                })
            }) =>
        {
            Ok(None)
        }
        Err(_) => Err(LinuxContainmentError::Identity(MEMBER_IDENTITY_DETAIL)),
    }
}

pub(super) fn pending_membership(
    started: &mut Option<Instant>,
    now: Instant,
) -> Result<Option<HashSet<LiveProcessIdentity>>, LinuxContainmentError> {
    let started = *started.get_or_insert(now);
    if now.saturating_duration_since(started) >= FREEZE_TIMEOUT {
        Err(LinuxContainmentError::Unavailable(
            MEMBERSHIP_TIMEOUT_DETAIL,
        ))
    } else {
        Ok(None)
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/linux/linux_containment_observation_tests.rs"]
mod tests;
