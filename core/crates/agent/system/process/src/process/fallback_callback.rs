use super::super::{
    ObservedProcess,
    constants::{MAX_PROCESS_ANCESTOR_DEPTH, MIN_PROCESS_PID},
};
use super::fallback_lineage::lineage_is_stable;
use super::fallback_observation::{
    ProcessIdentity, ProcessObservationApi, ProcessRow, observe_process,
};
use std::collections::HashSet;

#[cfg(target_os = "linux")]
pub(super) fn fenced_with_keys(
    root: crate::process::ProcessIdentity,
    peer_pid: u32,
    keys: &[&str],
) -> anyhow::Result<super::FencedDescendants> {
    if !root.matches() {
        return Ok(super::FencedDescendants::RootChanged);
    }
    let observed = observe_lineage(&super::fallback::SystemApi, root.pid, peer_pid, keys);
    if !root.matches() {
        return Ok(super::FencedDescendants::RootChanged);
    }
    Ok(super::FencedDescendants::Stable(
        observed.unwrap_or_default(),
    ))
}

/// Observe the authenticated callback peer's ancestry without requiring the
/// callback's terminal foreground group. Providers such as Qwen Code spawn
/// hook callbacks in their own session and PTY, so pid/start-time ancestry to
/// the custody shell is the fence rather than terminal group membership.
#[cfg(target_os = "linux")]
#[cfg(target_os = "linux")]
pub(super) fn fenced_ancestry_with_keys(
    root: crate::process::ProcessIdentity,
    peer_pid: u32,
    keys: &[&str],
) -> anyhow::Result<super::FencedDescendants> {
    if !root.matches() {
        return Ok(super::FencedDescendants::RootChanged);
    }
    let observed = observe_ancestry(&super::fallback::SystemApi, root, peer_pid, keys);
    if !root.matches() {
        return Ok(super::FencedDescendants::RootChanged);
    }
    Ok(super::FencedDescendants::Stable(
        observed.unwrap_or_default(),
    ))
}

fn observe_ancestry(
    api: &impl ProcessObservationApi,
    root: crate::process::ProcessIdentity,
    peer_pid: u32,
    keys: &[&str],
) -> Option<Vec<ObservedProcess>> {
    let lineage = capture_ancestry(api, peer_pid, root)?;
    let first = observe(api, &lineage, keys)?;
    if !lineage_is_stable(api, &lineage) {
        return None;
    }
    let second = observe(api, &lineage, keys)?;
    (first == second && lineage_is_stable(api, &lineage)).then_some(first)
}

fn capture_ancestry(
    api: &impl ProcessObservationApi,
    peer_pid: u32,
    root: crate::process::ProcessIdentity,
) -> Option<Vec<ProcessIdentity>> {
    if peer_pid < MIN_PROCESS_PID || peer_pid == root.pid {
        return None;
    }
    let mut current = peer_pid;
    let mut visited = HashSet::new();
    let mut lineage = Vec::new();
    loop {
        if lineage.len() >= MAX_PROCESS_ANCESTOR_DEPTH || !visited.insert(current) {
            return None;
        }
        let identity = api.identity(current)?;
        if identity.pid != current {
            return None;
        }
        let reached_root = current == root.pid;
        if reached_root && identity.start_time != root.start_time {
            return None;
        }
        current = identity.parent_pid;
        lineage.push(identity);
        if reached_root {
            return Some(lineage);
        }
    }
}

fn observe_lineage(
    api: &impl ProcessObservationApi,
    root_pid: u32,
    peer_pid: u32,
    keys: &[&str],
) -> Option<Vec<ObservedProcess>> {
    let root = api.identity(root_pid)?;
    if root.pid != root_pid || root.foreground_group <= i32::default() || peer_pid == root_pid {
        return None;
    }
    let lineage = capture(api, peer_pid, &root)?;
    let first = observe(api, &lineage, keys)?;
    if !lineage_is_stable(api, &lineage) {
        return None;
    }
    let second = observe(api, &lineage, keys)?;
    (first == second && lineage_is_stable(api, &lineage)).then_some(first)
}

fn capture(
    api: &impl ProcessObservationApi,
    peer_pid: u32,
    root: &ProcessIdentity,
) -> Option<Vec<ProcessIdentity>> {
    let mut current = peer_pid;
    let mut visited = HashSet::new();
    let mut lineage = Vec::new();
    loop {
        if current < MIN_PROCESS_PID
            || lineage.len() >= MAX_PROCESS_ANCESTOR_DEPTH
            || !visited.insert(current)
        {
            return None;
        }
        let identity = api.identity(current)?;
        if identity.pid != current || identity.foreground_group != root.foreground_group {
            return None;
        }
        let reached_root = current == root.pid;
        if reached_root && identity != *root {
            return None;
        }
        current = identity.parent_pid;
        lineage.push(identity);
        if reached_root {
            return Some(lineage);
        }
    }
}

fn observe(
    api: &impl ProcessObservationApi,
    lineage: &[ProcessIdentity],
    keys: &[&str],
) -> Option<Vec<ObservedProcess>> {
    let (_, descendants) = lineage.split_last()?;
    descendants
        .iter()
        .map(|identity| {
            observe_process(
                api,
                ProcessRow {
                    pid: identity.pid,
                    parent_pid: identity.parent_pid,
                    process_group: identity.process_group,
                    foreground_group: identity.foreground_group,
                    executable: identity.executable.clone(),
                    inventory_identity: Some(identity.clone()),
                },
                keys,
            )
        })
        .collect()
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../test_support/callback_fixture.rs"]
mod fixture;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/callback_lineage.rs"]
mod tests;
