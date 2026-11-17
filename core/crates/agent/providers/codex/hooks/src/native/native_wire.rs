use super::CodexHookInput;
use super::constants::{NATIVE_MAX_DATA_HOME_CANDIDATES, NATIVE_SCHEMA_VERSION};
use crate::agent::hooks::{HookEnvelope, HookIdentity};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) const NATIVE_INVALID: &str = "Codex native hook request is invalid";
pub(super) const NATIVE_ACK_INVALID: &str = "Codex native hook acknowledgement is invalid";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexNativeRequest {
    pub schema_version: u16,
    pub terminal_id: Uuid,
    pub runtime_epoch: Uuid,
    pub event_id: Uuid,
    pub hook_executable: PathBuf,
    pub data_home_candidates: Vec<PathBuf>,
    pub input: CodexHookInput,
}

impl CodexNativeRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == NATIVE_SCHEMA_VERSION
                && !self.terminal_id.is_nil()
                && !self.runtime_epoch.is_nil()
                && !self.event_id.is_nil()
                && !self.data_home_candidates.is_empty()
                && self.data_home_candidates.len() <= NATIVE_MAX_DATA_HOME_CANDIDATES,
            NATIVE_INVALID
        );
        for path in std::iter::once(&self.hook_executable).chain(&self.data_home_candidates) {
            crate::paths::validation::validate_absolute(path)?;
        }
        ensure!(
            serde_json::to_vec(&self.input)?.len() <= super::MAX_INPUT_BYTES,
            NATIVE_INVALID
        );
        self.input.validate()
    }

    pub fn envelope(&self, identity: &HookIdentity) -> Result<HookEnvelope> {
        self.validate()?;
        ensure!(
            identity.terminal_id == self.terminal_id
                && identity.runtime_epoch == self.runtime_epoch,
            NATIVE_INVALID
        );
        super::mapping::map_native(&self.input, identity, self.event_id)
    }

    pub fn cwd(&self) -> &Path {
        self.input.common().cwd
    }

    pub fn provider_session_id(&self) -> &str {
        self.input.common().session_id
    }

    pub fn is_session_start(&self) -> bool {
        matches!(self.input, CodexHookInput::SessionStart { .. })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexNativeAckCode {
    Accepted,
    Replay,
    Invalid,
    IdentityMismatch,
    AuthorityRevoked,
    ProcessUnavailable,
    NotReady,
    ProtocolError,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexNativeAck {
    pub schema_version: u16,
    pub event_id: Uuid,
    pub accepted: bool,
    pub code: CodexNativeAckCode,
}

impl CodexNativeAck {
    pub fn accepted(request: &CodexNativeRequest, replay: bool) -> Self {
        Self {
            schema_version: NATIVE_SCHEMA_VERSION,
            event_id: request.event_id,
            accepted: true,
            code: if replay {
                CodexNativeAckCode::Replay
            } else {
                CodexNativeAckCode::Accepted
            },
        }
    }

    pub fn rejected(request: &CodexNativeRequest, code: CodexNativeAckCode) -> Self {
        Self {
            schema_version: NATIVE_SCHEMA_VERSION,
            event_id: request.event_id,
            accepted: false,
            code,
        }
    }

    pub fn validate_for(&self, request: &CodexNativeRequest) -> Result<()> {
        ensure!(
            self.schema_version == NATIVE_SCHEMA_VERSION
                && self.event_id == request.event_id
                && self.accepted
                    == matches!(
                        self.code,
                        CodexNativeAckCode::Accepted | CodexNativeAckCode::Replay
                    ),
            NATIVE_ACK_INVALID
        );
        Ok(())
    }
}
