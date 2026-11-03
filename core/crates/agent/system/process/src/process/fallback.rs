use super::super::argv::process_argv;
use super::super::constants::{MAX_PROCESS_EVIDENCE_PATHS, MAX_PROCESS_FILE_DESCRIPTORS};
use super::super::process_environment::{parse_selected_environment, read_process_environment};
use super::super::ObservedProcess;
use super::fallback_lineage::{
    captured_lineage, identity_index, is_foreground_candidate, lineage_is_stable,
};
use super::fallback_observation::{
    ProcessEnvironmentObservation, ProcessIdentity, ProcessIdentityApi, ProcessObservationApi,
    observe_process,
};
use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;

pub(super) fn descendants_with_keys(
    root_pid: u32,
    environment_keys: &[&str],
) -> Result<Vec<ObservedProcess>> {
    let pids = super::fallback_inventory::inventory_pids()?;
    if !pids.contains(&root_pid) {
        return Ok(Vec::new());
    }
    let Some(root) = super::fallback_linux_identity::process_identity(root_pid) else {
        return Ok(Vec::new());
    };
    let mut rows = vec![super::fallback_linux_identity::process_row_from_identity(
        root.clone(),
    )];
    rows.extend(
        pids.into_iter()
            .filter(|pid| *pid != root_pid)
            .filter_map(super::fallback_linux_identity::process_identity)
            .map(super::fallback_linux_identity::process_row_from_identity),
    );
    let Some(identities) = identity_index(&rows) else {
        return Ok(Vec::new());
    };
    let mut observed = Vec::new();
    for process in rows {
        if !is_foreground_candidate(&process, &root) {
            continue;
        }
        let Some(lineage) = captured_lineage(&identities, process.pid, &root) else {
            continue;
        };
        let Some(candidate) = observe_process(&SystemApi, process, &environment_keys) else {
            continue;
        };
        if lineage_is_stable(&SystemApi, &lineage) {
            observed.push(candidate);
        }
    }
    Ok(observed)
}

pub(super) struct SystemApi;

impl ProcessIdentityApi for SystemApi {
    fn identity(&self, pid: u32) -> Option<ProcessIdentity> {
        super::fallback_linux_identity::process_identity(pid)
    }
}

impl ProcessObservationApi for SystemApi {
    fn argv(&self, pid: u32) -> Result<Vec<String>> {
        process_argv(pid)
    }

    fn environment(&self, pid: u32, keys: &[&str]) -> Result<ProcessEnvironmentObservation> {
        let Ok(file) = File::open(crate::process::procfs::environment_path(pid)) else {
            return Ok(ProcessEnvironmentObservation::Unavailable);
        };
        let Ok(bytes) = read_process_environment(file) else {
            return Ok(ProcessEnvironmentObservation::Unavailable);
        };
        parse_selected_environment(&bytes, keys).map(ProcessEnvironmentObservation::Selected)
    }

    fn evidence_paths(&self, pid: u32) -> Option<Vec<PathBuf>> {
        process_evidence_paths(pid)
    }
}

fn process_evidence_paths(pid: u32) -> Option<Vec<PathBuf>> {
    use super::super::constants::JSONL_EXTENSION;
    let directory = crate::process::procfs::file_descriptors_path(pid);
    let entries = std::fs::read_dir(directory).ok()?;
    let mut paths = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= MAX_PROCESS_FILE_DESCRIPTORS {
            return None;
        }
        let Ok(path) = entry.and_then(|value| std::fs::read_link(value.path())) else {
            continue;
        };
        if path.extension().and_then(|value| value.to_str()) != Some(JSONL_EXTENSION) {
            continue;
        }
        if paths.contains(&path) {
            continue;
        }
        if paths.len() >= MAX_PROCESS_EVIDENCE_PATHS {
            return None;
        }
        paths.push(path);
    }
    paths.sort();
    Some(paths)
}
