use crate::{
    constants::*,
    record::{Entry, Process, Record, Subject},
};
use anyhow::{Result, ensure};
use yolo_agent_contract::{AgentBinding, AgentState};
use yolo_hub_identity::{AgentId, RuntimeEpoch, TerminalId};
use yolo_terminal_model::{RecoveryState, TerminalView};

pub(crate) fn subject(binding: &AgentBinding) -> Result<Subject> {
    for path in [&binding.data_home, &binding.evidence_path, &binding.cwd] {
        ensure!(
            path.len() <= MAX_PRIVATE_METADATA_BYTES
                && std::path::Path::new(path).is_absolute()
                && !path.chars().any(char::is_control),
            INVALID_BINDING
        );
    }
    Ok(Subject {
        terminal_id: TerminalId::from_uuid(binding.terminal_id)?,
        kind: binding.kind.as_str().into(),
        native_reference: binding.provider_session_id.clone(),
        data_home: binding.data_home.clone(),
        evidence_path: binding.evidence_path.clone(),
    })
}

pub(crate) fn process(binding: &AgentBinding) -> Option<Process> {
    binding
        .process_pid
        .zip(binding.process_start_time)
        .map(|(pid, start_time)| Process { pid, start_time })
}

pub(crate) fn select(
    record: &Record,
    subject: &Subject,
    runtime: RuntimeEpoch,
    binding: &AgentBinding,
    view: &TerminalView,
) -> Result<Option<AgentId>> {
    let Some(id) = record.latest.get(&subject.terminal_id) else {
        return Ok(None);
    };
    let current = record
        .entries
        .get(id)
        .ok_or_else(|| anyhow::anyhow!(INVALID_RECORD))?;
    ensure!(
        binding.updated_at_ms >= current.updated_at_ms,
        STALE_BINDING
    );
    if &current.subject != subject {
        return Ok(None);
    }
    let candidate = process(binding);
    let same_runtime = current.runtime == runtime;
    ensure!(
        !(current.source_exited
            && same_runtime
            && candidate.is_some()
            && candidate == current.process
            && !matches!(binding.state, AgentState::Exited | AgentState::Unknown)),
        EXITED_PROCESS_REAPPEARED
    );
    let stable_process = same_runtime && candidate == current.process;
    let lost_process = same_runtime
        && candidate.is_none()
        && matches!(binding.state, AgentState::Exited | AgentState::Unknown);
    let known_exit = current.source_exited && same_runtime;
    let explicit_resume = !same_runtime && view.manifest.recovery_state == RecoveryState::Resumed;
    Ok((stable_process || lost_process || known_exit || explicit_resume).then_some(*id))
}

pub(crate) fn same_execution(entry: &Entry, runtime: RuntimeEpoch, binding: &AgentBinding) -> bool {
    entry.runtime == runtime
        && (entry.process == process(binding)
            || (process(binding).is_none()
                && matches!(binding.state, AgentState::Exited | AgentState::Unknown)))
}
