use super::super::constants::{
    MAX_PROCESS_ENVIRONMENT_BYTES, MAX_PROCESS_ENVIRONMENT_KEYS,
    MAX_PROCESS_ENVIRONMENT_VALUE_BYTES, MAX_SINGLE_PROCESS_ARGV_BYTES, MIN_PROCESS_PID,
    process_error,
};
use super::super::ObservedProcess;
use super::limits::{
    MAX_PROCESS_SNAPSHOT_ARGV_ITEMS, MAX_PROCESS_SNAPSHOT_EVIDENCE_ITEMS,
    MAX_PROCESS_SNAPSHOT_PROCESSES,
};
use super::validation_budget::SnapshotBudget;
use super::wire::ObservedProcessWire;
use anyhow::Result;
use std::collections::BTreeSet;

pub(super) fn decode_processes(
    processes: Vec<ObservedProcessWire>,
    environment_keys: &[String],
) -> Result<Vec<ObservedProcess>> {
    validate_processes(&processes, environment_keys)?;
    Ok(processes.into_iter().map(Into::into).collect())
}

pub(super) fn validate_processes(
    processes: &[ObservedProcessWire],
    environment_keys: &[String],
) -> Result<()> {
    anyhow::ensure!(
        processes.len() <= MAX_PROCESS_SNAPSHOT_PROCESSES,
        process_error::SNAPSHOT_INVALID
    );
    let mut pids = BTreeSet::new();
    let mut budget = SnapshotBudget::default();
    let allowed_environment = environment_keys.iter().map(String::as_str).collect();
    let foreground_group = processes.first().map(|process| process.foreground_group);
    for process in processes {
        anyhow::ensure!(
            Some(process.foreground_group) == foreground_group,
            process_error::SNAPSHOT_INVALID
        );
        validate_process(process, &allowed_environment, &mut pids, &mut budget)?;
    }
    Ok(())
}

fn validate_process(
    process: &ObservedProcessWire,
    allowed_environment: &BTreeSet<&str>,
    pids: &mut BTreeSet<u32>,
    budget: &mut SnapshotBudget,
) -> Result<()> {
    anyhow::ensure!(
        valid_platform_pid(process.pid)
            && process.start_time != u64::default()
            && valid_platform_pid(process.parent_pid)
            && process.parent_pid != process.pid
            && pids.insert(process.pid),
        process_error::SNAPSHOT_INVALID
    );
    anyhow::ensure!(
        process.process_group > 0
            && process.foreground_group > 0
            && process.process_group == process.foreground_group,
        process_error::SNAPSHOT_INVALID
    );
    anyhow::ensure!(
        !process.executable.is_empty()
            && !process
                .executable
                .contains(super::super::constants::ENVIRONMENT_NUL)
            && std::path::Path::new(&process.executable).is_absolute(),
        process_error::SNAPSHOT_INVALID
    );
    budget.record_path(process.executable.len())?;
    anyhow::ensure!(
        process.argv.len() <= MAX_PROCESS_SNAPSHOT_ARGV_ITEMS
            && process
                .argv
                .iter()
                .all(|value| !value.contains(super::super::constants::ENVIRONMENT_NUL)),
        process_error::SNAPSHOT_INVALID
    );
    let argv_bytes = process
        .argv
        .iter()
        .try_fold(usize::default(), |total, value| {
            total.checked_add(value.len())
        })
        .ok_or_else(invalid_snapshot)?;
    let (environment_items, environment_bytes) =
        validate_environment(process, allowed_environment)?;
    let process_argument_bytes = argv_bytes
        .checked_add(environment_bytes)
        .ok_or_else(invalid_snapshot)?;
    anyhow::ensure!(
        process_argument_bytes <= MAX_SINGLE_PROCESS_ARGV_BYTES,
        process_error::SNAPSHOT_INVALID
    );
    budget.record_arguments(process.argv.len(), process_argument_bytes)?;
    budget.record_environment(environment_items)?;
    if let Some(cwd) = &process.cwd {
        anyhow::ensure!(
            cwd.is_absolute() && !cwd.contains_nul(),
            process_error::SNAPSHOT_INVALID
        );
        budget.record_path(cwd.byte_len())?;
    }
    if let Some(paths) = &process.evidence_paths {
        anyhow::ensure!(
            paths.len() <= MAX_PROCESS_SNAPSHOT_EVIDENCE_ITEMS,
            process_error::SNAPSHOT_INVALID
        );
        budget.record_evidence(paths.len())?;
        for path in paths {
            anyhow::ensure!(
                path.is_absolute() && !path.contains_nul(),
                process_error::SNAPSHOT_INVALID
            );
            budget.record_path(path.byte_len())?;
        }
    }
    Ok(())
}

fn validate_environment(
    process: &ObservedProcessWire,
    allowed: &BTreeSet<&str>,
) -> Result<(usize, usize)> {
    let Some(environment) = &process.environment else {
        return Ok((usize::default(), usize::default()));
    };
    anyhow::ensure!(
        environment.len() <= MAX_PROCESS_ENVIRONMENT_KEYS,
        process_error::SNAPSHOT_INVALID
    );
    let mut bytes = usize::default();
    for (key, value) in environment {
        anyhow::ensure!(
            !key.is_empty()
                && allowed.contains(key.as_str())
                && !key.contains(super::super::constants::ENVIRONMENT_NUL)
                && !value.contains(super::super::constants::ENVIRONMENT_NUL)
                && value.len() <= MAX_PROCESS_ENVIRONMENT_VALUE_BYTES,
            process_error::SNAPSHOT_INVALID
        );
        bytes = bytes
            .checked_add(key.len())
            .and_then(|total| total.checked_add(value.len()))
            .ok_or_else(invalid_snapshot)?;
    }
    anyhow::ensure!(
        bytes <= MAX_PROCESS_ENVIRONMENT_BYTES,
        process_error::SNAPSHOT_INVALID
    );
    Ok((environment.len(), bytes))
}

pub(super) fn valid_platform_pid(pid: u32) -> bool {
    pid >= MIN_PROCESS_PID && i32::try_from(pid).is_ok()
}

pub(super) fn invalid_snapshot() -> anyhow::Error {
    anyhow::anyhow!(process_error::SNAPSHOT_INVALID)
}
