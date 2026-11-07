use super::constants::{errors, permission_modes, values};
use super::wire::CodexHookInput;
use anyhow::{Result, ensure};
use std::path::Path;

impl CodexHookInput {
    pub fn validate(&self) -> Result<()> {
        let common = self.common();
        ensure!(valid_identifier(common.session_id), errors::INPUT_INVALID);
        validate_metadata_path(common.cwd)?;
        if let Some(path) = common.transcript_path {
            validate_metadata_path(path)?;
        }
        for value in [
            self.turn_id(),
            self.agent_id(),
            self.agent_type(),
            self.tool_name(),
            self.tool_use_id(),
        ]
        .into_iter()
        .flatten()
        {
            ensure!(valid_identifier(value), errors::INPUT_INVALID);
        }
        validate_event_values(self)
    }
}

fn validate_event_values(input: &CodexHookInput) -> Result<()> {
    match input {
        CodexHookInput::SessionStart {
            permission_mode,
            source,
            ..
        } => {
            validate_permission(permission_mode)?;
            ensure!(
                values::START_SOURCES.contains(&source.as_str()),
                errors::INPUT_INVALID
            );
        }
        CodexHookInput::SessionEnd { reason, .. } => {
            ensure!(reason == values::OTHER, errors::INPUT_INVALID);
        }
        CodexHookInput::PreCompact { trigger, .. }
        | CodexHookInput::PostCompact { trigger, .. } => {
            ensure!(
                values::COMPACT_TRIGGERS.contains(&trigger.as_str()),
                errors::INPUT_INVALID
            );
        }
        CodexHookInput::SubagentStop {
            permission_mode,
            agent_transcript_path,
            ..
        } => {
            validate_permission(permission_mode)?;
            if let Some(path) = agent_transcript_path {
                validate_metadata_path(path)?;
            }
        }
        CodexHookInput::UserPromptSubmit {
            permission_mode, ..
        }
        | CodexHookInput::PreToolUse {
            permission_mode, ..
        }
        | CodexHookInput::PermissionRequest {
            permission_mode, ..
        }
        | CodexHookInput::PostToolUse {
            permission_mode, ..
        }
        | CodexHookInput::SubagentStart {
            permission_mode, ..
        }
        | CodexHookInput::Stop {
            permission_mode, ..
        } => validate_permission(permission_mode)?,
    }
    Ok(())
}

fn validate_permission(value: &str) -> Result<()> {
    ensure!(
        permission_modes::ALL.contains(&value),
        errors::INPUT_INVALID
    );
    Ok(())
}

fn validate_metadata_path(path: &Path) -> Result<()> {
    crate::paths::validation::validate_absolute(path)
        .map_err(|_| anyhow::anyhow!(errors::INPUT_INVALID))?;
    ensure!(
        path.as_os_str().as_encoded_bytes().len() <= crate::constants::MAX_RUNTIME_PATH_BYTES,
        errors::INPUT_INVALID
    );
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= crate::agent::hooks::MAX_HOOK_IDENTIFIER_BYTES
}
