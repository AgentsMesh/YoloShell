use super::super::argv::{ArgvSnapshot, process_argv_bounded};
use super::super::constants::{
    MAX_FOREGROUND_PROCESS_IDS, MAX_PROCESS_ANCESTOR_DEPTH, MAX_PROCESS_FILE_DESCRIPTORS,
};
use super::super::ObservedProcess;
use crate::process::macos_native::{self, ProcessMetadata};
use anyhow::{Result, ensure};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::budget::ScanBudget;

pub(super) trait ProcessApi {
    fn metadata(&self, pid: u32) -> Result<ProcessMetadata>;
    fn group_pids(&self, group: i32, limit: usize) -> Result<Vec<u32>>;
    fn executable(&self, pid: u32) -> Result<PathBuf>;
    fn cwd(&self, pid: u32) -> Result<PathBuf>;
    fn argv(&self, pid: u32, byte_limit: usize, environment_keys: &[&str]) -> Result<ArgvSnapshot>;
    fn evidence_paths(&self, pid: u32) -> Result<Vec<PathBuf>>;
}

pub(super) struct SystemApi;

impl ProcessApi for SystemApi {
    fn metadata(&self, pid: u32) -> Result<ProcessMetadata> {
        macos_native::process_metadata(pid)
    }

    fn group_pids(&self, group: i32, limit: usize) -> Result<Vec<u32>> {
        macos_native::foreground_group_pids(group, limit)
    }

    fn executable(&self, pid: u32) -> Result<PathBuf> {
        macos_native::executable_path(pid)
    }

    fn cwd(&self, pid: u32) -> Result<PathBuf> {
        macos_native::cwd_path(pid)
    }

    fn argv(&self, pid: u32, byte_limit: usize, environment_keys: &[&str]) -> Result<ArgvSnapshot> {
        process_argv_bounded(pid, byte_limit, environment_keys)
    }

    fn evidence_paths(&self, pid: u32) -> Result<Vec<PathBuf>> {
        super::macos_evidence::paths(pid, MAX_PROCESS_FILE_DESCRIPTORS)
    }
}

pub(super) fn descendants_with_keys(
    root_pid: u32,
    environment_keys: &[&str],
) -> Result<Vec<ObservedProcess>> {
    descendants_with_environment_keys(
        &SystemApi,
        root_pid,
        ScanBudget::default(),
        environment_keys,
    )
}


#[cfg(test)]
fn descendants_with(
    api: &impl ProcessApi,
    root_pid: u32,
    budget: ScanBudget,
) -> Result<Vec<ObservedProcess>> {
    descendants_with_environment_keys(api, root_pid, budget, &[])
}

fn descendants_with_environment_keys(
    api: &impl ProcessApi,
    root_pid: u32,
    mut budget: ScanBudget,
    environment_keys: &[&str],
) -> Result<Vec<ObservedProcess>> {
    let shell = api.metadata(root_pid)?;
    ensure!(shell.pid == root_pid, "shell metadata changed process id");
    if shell.foreground_group <= 0 {
        return Ok(Vec::new());
    }
    let pids = api.group_pids(shell.foreground_group, MAX_FOREGROUND_PROCESS_IDS)?;
    budget.reserve_pids(pids.len())?;
    let mut metadata = foreground_metadata(api, &pids, shell.foreground_group);
    let mut ancestry = HashMap::from([(root_pid, true)]);
    let mut observed = Vec::new();
    for pid in pids.into_iter().filter(|pid| *pid != root_pid) {
        let Some(info) = metadata.get(&pid).copied() else {
            continue;
        };
        if !is_descendant(api, info, &mut metadata, &mut ancestry, &mut budget)? {
            continue;
        }
        let Some(process) =
            super::macos_observe::observe_process(api, info, &mut budget, environment_keys)?
        else {
            continue;
        };
        if api.metadata(pid).ok() == Some(info) {
            observed.push(process);
        }
    }
    if !observed.is_empty() && api.metadata(root_pid).ok() != Some(shell) {
        return Ok(Vec::new());
    }
    Ok(observed)
}

fn foreground_metadata(
    api: &impl ProcessApi,
    pids: &[u32],
    foreground_group: i32,
) -> HashMap<u32, ProcessMetadata> {
    pids.iter()
        .filter_map(|pid| api.metadata(*pid).ok())
        .filter(|info| {
            info.pid != 0
                && info.process_group == foreground_group
                && info.foreground_group == foreground_group
        })
        .map(|info| (info.pid, info))
        .collect()
}

fn is_descendant(
    api: &impl ProcessApi,
    candidate: ProcessMetadata,
    metadata: &mut HashMap<u32, ProcessMetadata>,
    known: &mut HashMap<u32, bool>,
    budget: &mut ScanBudget,
) -> Result<bool> {
    let mut current = candidate.pid;
    let mut chain = Vec::new();
    let mut visited = HashSet::new();
    let outcome = loop {
        budget.record_ancestor_step()?;
        if let Some(outcome) = known.get(&current).copied() {
            break outcome;
        }
        if chain.len() >= MAX_PROCESS_ANCESTOR_DEPTH || !visited.insert(current) {
            break false;
        }
        chain.push(current);
        let Some(info) = metadata
            .get(&current)
            .copied()
            .or_else(|| api.metadata(current).ok())
        else {
            break false;
        };
        metadata.entry(current).or_insert(info);
        if info.parent_pid == current || info.parent_pid == 0 {
            break false;
        }
        current = info.parent_pid;
    };
    for pid in chain {
        known.insert(pid, outcome);
    }
    Ok(outcome)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/macos_observation_tests.rs"]
mod observation_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/macos_system_tests.rs"]
mod system_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../integration_tests/process/macos_support_tests.rs"]
mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/macos_tests.rs"]
mod tests;
