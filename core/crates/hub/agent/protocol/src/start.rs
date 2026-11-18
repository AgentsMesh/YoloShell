use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{AgentRunSelector, AgentRunSummary, validate_selector};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::{MAX_DIRECTORY_BYTES, MAX_TERMINAL_NAME_BYTES, TerminalSize};

use crate::{AGENT_PROTOCOL_SCHEMA_VERSION, MAX_PROVIDER_KIND_BYTES, ProtocolError};

/// Start a provider in a new terminal through a closed launch plan. The Hub
/// accepts only a provider id, never argv or environment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AgentStartFields", into = "AgentStartFields")]
pub struct AgentStartRequest(AgentStartFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStartFields {
    pub schema_version: u16,
    #[serde(serialize_with = "serialize_id", deserialize_with = "deserialize_id")]
    pub request_id: Uuid,
    #[serde(serialize_with = "serialize_id", deserialize_with = "deserialize_id")]
    pub start_token: Uuid,
    pub provider_id: String,
    pub name: Option<String>,
    pub cwd: String,
    pub size: TerminalSize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<Uuid>,
    /// Run that requested this start. Resolved once and stored as an exact
    /// address; it never grants authority over the parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<AgentRunSelector>,
}

impl AgentStartRequest {
    pub fn new(fields: AgentStartFields) -> Result<Self, ProtocolError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &AgentStartFields {
        &self.0
    }
}

impl TryFrom<AgentStartFields> for AgentStartRequest {
    type Error = ProtocolError;

    fn try_from(fields: AgentStartFields) -> Result<Self, Self::Error> {
        if fields.schema_version != AGENT_PROTOCOL_SCHEMA_VERSION {
            return Err(ProtocolError::UnsupportedSchema);
        }
        if fields.request_id.is_nil() || fields.start_token.is_nil() {
            return Err(ProtocolError::InvalidRequest);
        }
        if !metadata(&fields.cwd, MAX_DIRECTORY_BYTES)
            || fields
                .name
                .as_ref()
                .is_some_and(|name| !metadata(name, MAX_TERMINAL_NAME_BYTES))
        {
            return Err(ProtocolError::InvalidRequest);
        }
        if fields
            .workspace_id
            .is_some_and(|workspace| workspace.is_nil())
        {
            return Err(ProtocolError::InvalidRequest);
        }
        if let Some(parent) = &fields.parent {
            validate_selector(parent).map_err(|_| ProtocolError::InvalidRequest)?;
        }
        if !provider_id(&fields.provider_id) {
            return Err(ProtocolError::InvalidProvider);
        }
        Ok(Self(fields))
    }
}

impl From<AgentStartRequest> for AgentStartFields {
    fn from(value: AgentStartRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStartBinding {
    /// The terminal was created and launch input queued; the provider may not
    /// have emitted its first binding yet (Codex does so after the first prompt).
    Awaiting,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStartResult {
    pub address: TerminalAddress,
    pub provider_id: String,
    pub binding: AgentStartBinding,
    /// Present when the Hub has a durable run store; absent deployments keep
    /// the terminal-level start result only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<AgentRunSummary>,
}

fn deserialize_id<'de, D: Deserializer<'de>>(input: D) -> Result<Uuid, D::Error> {
    let value = String::deserialize(input)?;
    let parsed = Uuid::parse_str(&value).map_err(D::Error::custom)?;
    if parsed.to_string() != value || parsed.is_nil() {
        return Err(D::Error::custom(ProtocolError::InvalidRequest));
    }
    Ok(parsed)
}

fn serialize_id<S: Serializer>(value: &Uuid, output: S) -> Result<S::Ok, S::Error> {
    output.serialize_str(&value.to_string())
}

fn metadata(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.contains('\0')
}

fn provider_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROVIDER_KIND_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
