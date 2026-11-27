use super::budget::ScanBudget;
use super::macos::{ProcessApi, SystemApi};
use super::super::ObservedProcess;
use super::super::constants::MAX_PROCESS_ANCESTOR_DEPTH;
use anyhow::Result;
use std::collections::HashSet;

pub(super) fn ancestry_with_keys(
    root: crate::process::ProcessIdentity,
    peer_pid: u32,
    environment_keys: &[&str],
    include_root: bool,
) -> Result<super::FencedDescendants> {
    if !root.matches() {
        return Ok(super::FencedDescendants::RootChanged);
    }
    let observed = observation_ancestry(&SystemApi, root, peer_pid, environment_keys, include_root)
        .unwrap_or_default();
    if !root.matches() {
        return Ok(super::FencedDescendants::RootChanged);
    }
    Ok(super::FencedDescendants::Stable(observed))
}

/// Observe callbacks that run in their own session and PTY. The pid/start-time
/// chain must reach the exact custody shell; no foreground-group agreement is
/// required because such callbacks have their own controlling terminal.
fn observation_ancestry(
    api: &impl ProcessApi,
    root: crate::process::ProcessIdentity,
    peer_pid: u32,
    environment_keys: &[&str],
    include_root: bool,
) -> Result<Vec<ObservedProcess>> {
    if peer_pid == root.pid {
        return Ok(Vec::new());
    }
    let mut budget = ScanBudget::default();
    let mut current = peer_pid;
    let mut visited = HashSet::new();
    let mut lineage = Vec::new();
    loop {
        if lineage.len() >= MAX_PROCESS_ANCESTOR_DEPTH || !visited.insert(current) {
            return Ok(Vec::new());
        }
        let Ok(info) = api.metadata(current) else {
            return Ok(Vec::new());
        };
        if info.pid != current {
            return Ok(Vec::new());
        }
        let reached_root = current == root.pid;
        if reached_root && info.start_time != root.start_time {
            return Ok(Vec::new());
        }
        lineage.push(info);
        if reached_root {
            break;
        }
        current = info.parent_pid;
    }
    budget.reserve_pids(lineage.len())?;
    let Some((root_info, descendants)) = lineage.split_last() else {
        return Ok(Vec::new());
    };
    let subjects = if include_root { lineage.as_slice() } else { descendants };
    let mut observed = Vec::with_capacity(subjects.len());
    for info in subjects {
        let Some(process) =
            super::macos_observe::observe_process(api, *info, &mut budget, environment_keys)?
        else {
            return Ok(Vec::new());
        };
        if api.metadata(info.pid).ok() != Some(*info) {
            return Ok(Vec::new());
        }
        observed.push(process);
    }
    if api.metadata(root.pid).ok() != Some(*root_info) {
        return Ok(Vec::new());
    }
    Ok(observed)
}
