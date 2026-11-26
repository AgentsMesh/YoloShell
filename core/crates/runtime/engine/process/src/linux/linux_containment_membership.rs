use super::constants::{CGROUP_PROCESSES_FILE, PIDS_MAX_VALUE};
use super::io::{read_control, write_control};
use super::{LinuxContainmentError, LinuxContainmentLease};
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use std::collections::BTreeSet;

const ROOT_GONE_DETAIL: &str = "bootstrap process exited before cgroup migration";
const ROOT_CHANGED_DETAIL: &str = "bootstrap process changed during cgroup migration";
const MEMBERSHIP_MISMATCH_DETAIL: &str = "cgroup bootstrap membership verification failed";

impl LinuxContainmentLease {
    pub fn migrate_bootstrap(
        &mut self,
        bootstrap: ProcessIdentity,
    ) -> Result<(), LinuxContainmentError> {
        if self.bound_root.is_some() {
            return Err(LinuxContainmentError::Identity(
                "cgroup lease is already bound",
            ));
        }
        let live = LiveProcessIdentity::capture(bootstrap)
            .map_err(|_| LinuxContainmentError::Identity(ROOT_CHANGED_DETAIL))?
            .ok_or(LinuxContainmentError::Identity(ROOT_GONE_DETAIL))?;
        let pid_line = format!("{}\n", bootstrap.pid);
        write_control(&self.leaf, CGROUP_PROCESSES_FILE, &pid_line)?;
        let members = read_member_pids(&self.leaf)?;
        if members.len() != 1 || !members.contains(&bootstrap.pid) {
            return Err(LinuxContainmentError::Identity(MEMBERSHIP_MISMATCH_DETAIL));
        }
        if !live
            .try_matches()
            .map_err(|_| LinuxContainmentError::Identity(ROOT_CHANGED_DETAIL))?
        {
            return Err(LinuxContainmentError::Identity(ROOT_CHANGED_DETAIL));
        }
        self.bound_root = Some(bootstrap);
        self.record_bound_root(bootstrap);
        Ok(())
    }

    pub fn remove_empty(self) -> Result<(), LinuxContainmentError> {
        if !read_member_pids(&self.leaf)?.is_empty() {
            return Err(LinuxContainmentError::Identity(
                "cannot remove a populated session cgroup",
            ));
        }
        super::descriptor::remove_exact_leaf(&self.root, &self.leaf, &self.record.leaf_name)
    }
}

pub(super) fn read_member_pids(
    leaf: &std::fs::File,
) -> Result<BTreeSet<u32>, LinuxContainmentError> {
    let maximum = PIDS_MAX_VALUE
        .trim()
        .parse::<usize>()
        .map_err(|_| LinuxContainmentError::Identity("configured pids.max is invalid"))?;
    let mut members = BTreeSet::new();
    for line in read_control(leaf, CGROUP_PROCESSES_FILE)?.lines() {
        let pid = line.parse::<u32>().ok().filter(|pid| *pid != 0).ok_or(
            LinuxContainmentError::Identity("cgroup membership contains an invalid process id"),
        )?;
        members.insert(pid);
        if members.len() > maximum {
            return Err(LinuxContainmentError::Identity(
                "cgroup membership exceeds its bound",
            ));
        }
    }
    Ok(members)
}
