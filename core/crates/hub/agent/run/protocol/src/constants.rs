//! Wire constants and metadata validation for agent runs.

use crate::run::AgentRunSelector;
use std::collections::BTreeMap;
use yolo_hub_terminal_protocol::PublicError;

pub const AGENT_RUN_PROTOCOL_SCHEMA_VERSION: u16 = 1;
pub const MAX_AGENT_RUN_NAME_BYTES: usize = 128;
pub const MAX_AGENT_RUN_ALIASES: usize = 16;
pub const MAX_AGENT_RUN_ALIAS_BYTES: usize = 128;
pub const MAX_AGENT_RUN_LABELS: usize = 32;
pub const MAX_AGENT_RUN_LABEL_KEY_BYTES: usize = 64;
pub const MAX_AGENT_RUN_LABEL_VALUE_BYTES: usize = 256;
pub const MAX_AGENT_RUN_PAGE: usize = 128;
/// Launched runs (terminal exists) across one Hub. Stopped runs free a slot.
pub const MAX_ACTIVE_AGENT_RUNS: usize = 128;
/// Launched direct children under one parent run.
pub const MAX_AGENT_RUN_CHILDREN: usize = 32;
/// Longest parent chain below a root run (a root has depth 0).
pub const MAX_AGENT_RUN_DEPTH: usize = 16;
pub const FIRST_AGENT_RUN_REVISION: u64 = 1;
pub const MAX_PROVIDER_ID_BYTES: usize = 64;
/// Central declaration for the `AgentRunFilter` serde tag.
pub const AGENT_RUN_FILTER_TAG: &str = "field";

pub fn validate_selector(selector: &AgentRunSelector) -> Result<(), PublicError> {
    match selector {
        AgentRunSelector::Address(address) => {
            if address.run_id.as_uuid().is_nil() {
                return Err(PublicError::InvalidRequest);
            }
        }
        AgentRunSelector::Name(name) => validate_name(name)?,
    }
    Ok(())
}

pub fn validate_name(value: &str) -> Result<(), PublicError> {
    if value.trim().is_empty() || value.len() > MAX_AGENT_RUN_NAME_BYTES || value.contains('\0') {
        return Err(PublicError::InvalidMetadata);
    }
    Ok(())
}

pub fn validate_aliases(aliases: &[String]) -> Result<(), PublicError> {
    if aliases.len() > MAX_AGENT_RUN_ALIASES {
        return Err(PublicError::InvalidMetadata);
    }
    for alias in aliases {
        if alias.is_empty()
            || alias.len() > MAX_AGENT_RUN_ALIAS_BYTES
            || alias.contains('\0')
            || alias.trim().is_empty()
        {
            return Err(PublicError::InvalidMetadata);
        }
    }
    Ok(())
}

pub fn validate_labels(labels: &BTreeMap<String, String>) -> Result<(), PublicError> {
    if labels.len() > MAX_AGENT_RUN_LABELS {
        return Err(PublicError::InvalidMetadata);
    }
    for (key, value) in labels {
        validate_label(key, value)?;
    }
    Ok(())
}

pub fn validate_label(key: &str, value: &str) -> Result<(), PublicError> {
    if key.is_empty()
        || key.len() > MAX_AGENT_RUN_LABEL_KEY_BYTES
        || value.len() > MAX_AGENT_RUN_LABEL_VALUE_BYTES
        || key.contains('\0')
        || value.contains('\0')
    {
        return Err(PublicError::InvalidMetadata);
    }
    Ok(())
}

pub fn validate_provider_id(value: &str) -> Result<(), PublicError> {
    if value.is_empty()
        || value.len() > MAX_PROVIDER_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(PublicError::InvalidMetadata);
    }
    Ok(())
}
