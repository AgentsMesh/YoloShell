use super::{KernelPidfdOperations, PidfdOperations, RetainedPidfd, identity};
use crate::process::live_identity::ProcessExecutionState;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const PROCESS_STATE_CONTEXT: &str = "observe exact Linux process stop state";
const PROCESS_STAT_FILE: &str = "stat";
const PROCESS_TASK_DIRECTORY: &str = "task";
const PROC_ROOT: &str = "/proc";
const STAT_COMMAND_END: &str = ") ";
const STAT_START_TIME_INDEX_AFTER_COMMAND: usize = 19;
const STOPPED_JOB_CONTROL_STATE: u8 = b'T';
const STOPPED_TRACED_STATE: u8 = b't';
const EMPTY_TASK_SET_DETAIL: &str = "live Linux process exposed no task identities";
const STAT_FORMAT_DETAIL: &str = "Linux process stat record has an invalid format";
const STAT_START_TIME_DETAIL: &str = "Linux process stat record has no start time";

pub(super) fn execution_state(
    retained: &RetainedPidfd,
    pid: u32,
    generation: u64,
) -> Result<ProcessExecutionState> {
    if retained.identity != identity(pid, generation) {
        return Ok(ProcessExecutionState::Gone);
    }
    if exited(retained)? {
        return Ok(ProcessExecutionState::Gone);
    }
    match observe_live_process(retained, pid, generation) {
        Ok(state) => Ok(state),
        Err(error) if super::error_is_process_gone(&error) => {
            classify_missing_procfs(retained, error)
        }
        Err(error) => Err(error),
    }
}

fn observe_live_process(
    retained: &RetainedPidfd,
    pid: u32,
    generation: u64,
) -> Result<ProcessExecutionState> {
    let process = process_path(pid);
    let first_start = read_start_time(&process.join(PROCESS_STAT_FILE))?;
    if first_start != generation {
        return classify_identity_change(retained);
    }
    let first_tasks = read_task_states(&process.join(PROCESS_TASK_DIRECTORY))?;
    let second_tasks = read_task_states(&process.join(PROCESS_TASK_DIRECTORY))?;
    let second_start = read_start_time(&process.join(PROCESS_STAT_FILE))?;
    if second_start != generation || exited(retained)? {
        return classify_identity_change(retained);
    }
    ensure!(!first_tasks.is_empty(), EMPTY_TASK_SET_DETAIL);
    Ok(classify_task_states(&first_tasks, &second_tasks))
}

fn classify_missing_procfs(
    retained: &RetainedPidfd,
    error: anyhow::Error,
) -> Result<ProcessExecutionState> {
    if exited(retained)? {
        Ok(ProcessExecutionState::Gone)
    } else {
        Err(error)
    }
}

fn exited(retained: &RetainedPidfd) -> Result<bool> {
    KernelPidfdOperations
        .exited(&retained.descriptor)
        .context(PROCESS_STATE_CONTEXT)
}

fn classify_identity_change(retained: &RetainedPidfd) -> Result<ProcessExecutionState> {
    if exited(retained)? {
        Ok(ProcessExecutionState::Gone)
    } else {
        anyhow::bail!(PROCESS_STATE_CONTEXT)
    }
}

fn process_path(pid: u32) -> PathBuf {
    Path::new(PROC_ROOT).join(pid.to_string())
}

fn read_start_time(path: &Path) -> Result<u64> {
    let record = fs::read(path).with_context(|| format!("{PROCESS_STATE_CONTEXT}: {path:?}"))?;
    let fields = fields_after_command(&record)?;
    let value = fields
        .get(STAT_START_TIME_INDEX_AFTER_COMMAND)
        .context(STAT_START_TIME_DETAIL)?;
    std::str::from_utf8(value)?.parse().map_err(Into::into)
}

fn read_task_states(path: &Path) -> Result<BTreeSet<(u32, u8)>> {
    let mut states = BTreeSet::new();
    for entry in fs::read_dir(path).with_context(|| format!("{PROCESS_STATE_CONTEXT}: {path:?}"))? {
        let entry = entry?;
        let Some(task) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok())
        else {
            continue;
        };
        let record = fs::read(entry.path().join(PROCESS_STAT_FILE))?;
        let state = fields_after_command(&record)?
            .first()
            .and_then(|value| value.first())
            .copied()
            .context(STAT_FORMAT_DETAIL)?;
        states.insert((task, state));
    }
    Ok(states)
}

fn fields_after_command(record: &[u8]) -> Result<Vec<&[u8]>> {
    let boundary = record
        .windows(STAT_COMMAND_END.len())
        .rposition(|window| window == STAT_COMMAND_END.as_bytes())
        .context(STAT_FORMAT_DETAIL)?;
    Ok(record[boundary + STAT_COMMAND_END.len()..]
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|field| !field.is_empty())
        .collect())
}

fn all_stopped(states: &BTreeSet<(u32, u8)>) -> bool {
    states
        .iter()
        .all(|(_task, state)| matches!(*state, STOPPED_JOB_CONTROL_STATE | STOPPED_TRACED_STATE))
}

fn classify_task_states(
    first: &BTreeSet<(u32, u8)>,
    second: &BTreeSet<(u32, u8)>,
) -> ProcessExecutionState {
    if first == second && all_stopped(first) {
        ProcessExecutionState::Stopped
    } else {
        ProcessExecutionState::Running
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/live_identity/linux_pidfd/stop_tests.rs"]
mod tests;
