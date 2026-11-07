use crate::constants::{self, message};
use crate::{AgentAdapter, DetectionContext};
use anyhow::{Context, Result, ensure};
use std::path::{Component, Path};
use yolo_agent_contract::{AgentBinding, AgentKind, AgentState};

/// Display titles are single-line text without terminal controls or
/// bidirectional overrides. They are never used as identity. The terminal
/// metadata protocol owns this policy because the title is injected into the
/// same PTY presentation path as OSC titles.
pub fn valid_display_title(text: &str) -> bool {
    yolo_terminal_metadata_protocol::valid_text(
        text,
        yolo_terminal_metadata_protocol::MAX_TITLE_BYTES,
    )
}

pub fn validate_launch_environment(environment: &crate::LaunchEnvironment) -> Result<()> {
    ensure!(
        environment.len() <= constants::MAX_PROVIDER_ENVIRONMENT_VARIABLES,
        message::ENVIRONMENT_LIMIT
    );
    let mut bytes = 0_usize;
    for (key, value) in environment {
        ensure!(
            !key.is_empty()
                && !key.contains(constants::ENVIRONMENT_ASSIGNMENT)
                && !key.contains(constants::ENVIRONMENT_NUL)
                && !key.starts_with(constants::RESERVED_ENVIRONMENT_PREFIX)
                && !value.contains(constants::ENVIRONMENT_NUL),
            message::ENVIRONMENT_INVALID
        );
        bytes = bytes.saturating_add(key.len()).saturating_add(value.len());
    }
    ensure!(
        bytes <= constants::MAX_PROVIDER_ENVIRONMENT_BYTES,
        message::ENVIRONMENT_LIMIT
    );
    Ok(())
}

pub fn validate_core(binding: &AgentBinding, context: &DetectionContext) -> Result<()> {
    ensure!(
        binding.schema_version == constants::BINDING_SCHEMA_VERSION,
        message::SCHEMA
    );
    ensure!(
        binding.terminal_id == context.terminal_id,
        message::YOLO_SESSION
    );
    let provider_session_id = binding.provider_session_id.as_str();
    ensure!(
        !provider_session_id.is_empty() && provider_session_id.trim() == provider_session_id,
        message::AGENT_SESSION_ID
    );
    ensure!(
        binding.detected_at_ms >= 0 && binding.updated_at_ms >= binding.detected_at_ms,
        message::TIMESTAMPS
    );
    match binding.state {
        AgentState::Exited => {
            ensure!(binding.process_pid.is_none(), message::PROCESS_PID);
            ensure!(
                binding.process_start_time.is_none(),
                message::PROCESS_START_TIME
            );
        }
        AgentState::Unknown
            if binding.process_pid.is_none()
                && binding.process_start_time.is_none()
                && binding.recovery_availability
                    == yolo_agent_contract::AgentRecoveryAvailability::Available => {}
        _ => {
            ensure!(
                binding
                    .process_pid
                    .is_some_and(|pid| pid >= constants::MIN_PROCESS_PID),
                message::PROCESS_PID
            );
            ensure!(
                binding.process_start_time.is_some_and(|start| start != 0),
                message::PROCESS_START_TIME
            );
        }
    }
    Ok(())
}

pub fn validate_evidence_binding(
    adapter: &dyn AgentAdapter,
    binding: &AgentBinding,
    context: &DetectionContext,
) -> Result<()> {
    validate_evidence_ownership(adapter, binding, context)?;
    let binding_context = context.with_binding_data_home(
        binding.kind.clone(),
        Path::new(&binding.data_home).to_path_buf(),
    );
    let path = Path::new(&binding.evidence_path);
    let evidence = adapter
        .inspect_evidence_in_context(path, Path::new(&binding.cwd), &binding_context)
        .context(message::EVIDENCE_READ)?
        .ok_or_else(|| anyhow::anyhow!(message::STALE_EVIDENCE))?;
    ensure!(
        evidence.provider_session_id == binding.provider_session_id,
        message::STALE_EVIDENCE
    );
    let cwd_matches = !adapter.requires_evidence_cwd_match()
        || same_directory(&evidence.cwd, Path::new(&binding.cwd));
    ensure!(
        same_directory(&evidence.path, path)
            && cwd_matches
            && same_directory(&evidence.data_home, Path::new(&binding.data_home)),
        message::STALE_EVIDENCE
    );
    Ok(())
}

pub fn validate_recovery_availability(
    adapter: &dyn AgentAdapter,
    binding: &AgentBinding,
) -> Result<()> {
    ensure!(
        adapter
            .session_generation_policy()
            .recovery_availability_is_valid(binding),
        message::RECOVERY_AVAILABILITY
    );
    Ok(())
}

pub fn validate_evidence_ownership(
    adapter: &dyn AgentAdapter,
    binding: &AgentBinding,
    context: &DetectionContext,
) -> Result<()> {
    let binding_context = context.with_binding_data_home(
        binding.kind.clone(),
        Path::new(&binding.data_home).to_path_buf(),
    );
    ensure!(
        adapter.owns_evidence_path(Path::new(&binding.evidence_path), &binding_context),
        message::EVIDENCE_PATH
    );
    Ok(())
}

pub fn validate_adapter_binding(
    binding: &AgentBinding,
    _context: &DetectionContext,
    kind: AgentKind,
    executable_names: &[&str],
    data_home: &Path,
    evidence_root: &Path,
) -> Result<()> {
    ensure!(binding.kind == kind, message::KIND);
    let executable = Path::new(&binding.executable)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&binding.executable);
    ensure!(
        executable_names.contains(&executable) || constants::NODE_EXECUTABLES.contains(&executable),
        message::EXECUTABLE
    );
    ensure!(Path::new(&binding.cwd).is_absolute(), message::CWD);
    ensure!(
        !binding.data_home.is_empty() && data_home.is_absolute(),
        message::DATA_HOME
    );
    ensure!(
        evidence_path_is_owned(Path::new(&binding.evidence_path), evidence_root),
        message::EVIDENCE_PATH
    );
    Ok(())
}

pub fn evidence_path_is_owned(path: &Path, root: &Path) -> bool {
    path.extension().and_then(|value| value.to_str()) == Some(constants::JSONL_EXTENSION)
        && path_is_within(path, root)
}

/// Evidence paths identify a directory, not a location below it. Canonical
/// comparison keeps symlinked provider homes from reporting stale evidence.
fn same_directory(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(test)]
#[path = "../unit_tests/validation/tests.rs"]
mod tests;

include!("path_validation.rs");
