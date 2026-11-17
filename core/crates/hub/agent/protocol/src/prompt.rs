//! Atomic agent prompt: submit one prompt and optionally wait for the turn to
//! leave the working state. The Hub acquires the writer lease, submits through
//! the native control path, releases the lease, and evaluates the wait from
//! observations rather than screen text.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_agent_input::Prompt;
use yolo_hub_identity::{AgentTarget, TerminalAddress};

use crate::{AGENT_PROTOCOL_SCHEMA_VERSION, AgentSnapshot, CommandReceipt, ProtocolError};

pub const MAX_PROMPT_WAIT_MS: u32 = 1_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "reference", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentPromptReference {
    /// Exact observed execution.
    Execution { target: AgentTarget },
    /// Current observed execution bound to this terminal.
    Terminal { address: TerminalAddress },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "wait", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentPromptWait {
    None,
    TurnEnd { timeout_ms: u32 },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AgentPromptFields", into = "AgentPromptFields")]
pub struct AgentPromptRequest(AgentPromptFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPromptFields {
    pub schema_version: u16,
    pub request_id: Uuid,
    pub reference: AgentPromptReference,
    pub prompt: Prompt,
    pub wait: AgentPromptWait,
    pub take_control: bool,
}

impl AgentPromptRequest {
    pub fn new(fields: AgentPromptFields) -> Result<Self, ProtocolError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &AgentPromptFields {
        &self.0
    }
}

impl TryFrom<AgentPromptFields> for AgentPromptRequest {
    type Error = ProtocolError;

    fn try_from(fields: AgentPromptFields) -> Result<Self, Self::Error> {
        if fields.schema_version != AGENT_PROTOCOL_SCHEMA_VERSION || fields.request_id.is_nil() {
            return Err(ProtocolError::InvalidRequest);
        }
        match &fields.reference {
            AgentPromptReference::Execution { target } => {
                if target.address.agent_id.as_uuid().is_nil()
                    || target.execution_id.as_uuid().is_nil()
                {
                    return Err(ProtocolError::InvalidRequest);
                }
            }
            AgentPromptReference::Terminal { address } => {
                if address.terminal_id.as_uuid().is_nil() {
                    return Err(ProtocolError::InvalidRequest);
                }
            }
        }
        if let AgentPromptWait::TurnEnd { timeout_ms } = fields.wait {
            if timeout_ms == 0 || timeout_ms > MAX_PROMPT_WAIT_MS {
                return Err(ProtocolError::InvalidRequest);
            }
        }
        Ok(Self(fields))
    }
}

impl From<AgentPromptRequest> for AgentPromptFields {
    fn from(value: AgentPromptRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPromptOutcome {
    /// Submission was accepted; no turn wait was requested.
    Delivered,
    /// A post-submit observation left the working state.
    TurnEnded,
    /// The post-submit observation is waiting for approval.
    Blocked,
    /// The bounded turn wait elapsed without a matching observation.
    TimedOut,
    /// The execution ended during the wait.
    Exited,
    /// Submission was rejected before any effect.
    Rejected,
    /// Submission may have happened but no receipt was observed.
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPromptResult {
    pub receipt: CommandReceipt,
    pub terminal: TerminalAddress,
    pub outcome: AgentPromptOutcome,
    /// Last observed snapshot used for the outcome; absent for a lost
    /// execution or a submission whose receipt was unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<AgentSnapshot>,
}
