//! Stop an agent run by ending its terminal. The effect is observed: the Hub
//! terminates the terminal and records the run as stopped only after Host
//! state confirms the runtime is no longer live.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AgentRunSelector, AgentRunSummary, validate_name,
};

use crate::{AGENT_PROTOCOL_SCHEMA_VERSION, ProtocolError};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AgentStopFields", into = "AgentStopFields")]
pub struct AgentStopRequest(AgentStopFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStopFields {
    pub schema_version: u16,
    pub request_id: Uuid,
    pub selector: AgentRunSelector,
    pub force: bool,
}

impl AgentStopRequest {
    pub fn new(fields: AgentStopFields) -> Result<Self, ProtocolError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &AgentStopFields {
        &self.0
    }
}

impl TryFrom<AgentStopFields> for AgentStopRequest {
    type Error = ProtocolError;

    fn try_from(fields: AgentStopFields) -> Result<Self, Self::Error> {
        if fields.schema_version != AGENT_PROTOCOL_SCHEMA_VERSION {
            return Err(ProtocolError::UnsupportedSchema);
        }
        if fields.request_id.is_nil() {
            return Err(ProtocolError::InvalidRequest);
        }
        match &fields.selector {
            AgentRunSelector::Address(address) => {
                if address.run_id.as_uuid().is_nil() {
                    return Err(ProtocolError::InvalidRequest);
                }
            }
            AgentRunSelector::Name(name) => {
                validate_name(name).map_err(|_| ProtocolError::InvalidRequest)?;
            }
        }
        Ok(Self(fields))
    }
}

impl From<AgentStopRequest> for AgentStopFields {
    fn from(value: AgentStopRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStopOutcome {
    /// The terminal was live and is now confirmed not live.
    Terminated,
    /// The terminal had already exited; the run is now recorded as stopped.
    AlreadyExited,
    /// The run was already recorded as stopped; no effect was produced.
    AlreadyStopped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentStopResult {
    pub run: AgentRunSummary,
    pub outcome: AgentStopOutcome,
}
