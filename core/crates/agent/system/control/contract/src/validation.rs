use crate::{ControlContractError, constants::*};
use std::path::{Path, PathBuf};
use yolo_agent_adapter_contract::{
    DetectionContext, constants::MIN_PROCESS_PID, validation::validate_core,
};
use yolo_agent_contract::{AgentBinding, AgentKind, ZERO_PROCESS_START_TIME};

pub(crate) fn provider(kind: &AgentKind) -> Result<(), ControlContractError> {
    let value = kind.as_str();
    let canonical = match kind {
        AgentKind::Codex | AgentKind::ClaudeCode => true,
        AgentKind::Other(name) => matches!(AgentKind::from_name(name), AgentKind::Other(_)),
    };
    if !value.is_empty()
        && value.len() <= MAX_PROVIDER_KIND_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
        && canonical
    {
        Ok(())
    } else {
        Err(ControlContractError::UnsupportedProvider)
    }
}
pub(crate) fn process(pid: u32, start: u64) -> Result<(), ControlContractError> {
    if (MIN_PROCESS_PID..=MAX_PROCESS_PID).contains(&pid) && start != ZERO_PROCESS_START_TIME {
        Ok(())
    } else {
        Err(ControlContractError::InvalidProcess)
    }
}
pub(crate) fn boot(value: &str) -> Result<(), ControlContractError> {
    if text(value, MAX_BOOT_ID_BYTES)
        && !value.chars().any(char::is_whitespace)
        && yolo_process_identity::BootIdentity::from_value(value).known()
    {
        Ok(())
    } else {
        Err(ControlContractError::InvalidBoot)
    }
}

pub(crate) fn binding(value: &AgentBinding) -> Result<(), ControlContractError> {
    provider(&value.kind)?;
    if value.terminal_id.is_nil() {
        return Err(ControlContractError::InvalidIdentity);
    }
    let context = DetectionContext::new(
        value.terminal_id,
        PathBuf::from(&value.cwd),
        std::iter::empty(),
    );
    validate_core(value, &context).map_err(|_| ControlContractError::InvalidBinding)?;
    if !text(&value.provider_session_id, MAX_BINDING_TEXT_BYTES)
        || !text(&value.executable, MAX_BINDING_TEXT_BYTES)
    {
        return Err(ControlContractError::InvalidBinding);
    }
    for path in [&value.cwd, &value.data_home, &value.evidence_path] {
        if !text(path, MAX_BINDING_TEXT_BYTES) || !Path::new(path).is_absolute() {
            return Err(ControlContractError::InvalidBinding);
        }
    }
    if let (Some(pid), Some(start)) = (value.process_pid, value.process_start_time) {
        process(pid, start)?;
    }
    Ok(())
}

fn text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}
