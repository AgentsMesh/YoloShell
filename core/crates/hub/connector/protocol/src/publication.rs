use crate::{ConnectorError, ReportedState, validation};
use serde::{Deserialize, Serialize};
use yolo_hub_agent_protocol::ObservationCursor;
use yolo_hub_identity::{AgentTarget, ConnectorGeneration, ConnectorId, RuntimeTarget};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PublicationWire")]
pub struct Publication {
    pub connector_id: ConnectorId,
    pub generation: ConnectorGeneration,
    pub agent: AgentTarget,
    pub terminal: RuntimeTarget,
    pub sequence: u64,
    pub state: ReportedState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicationWire {
    connector_id: ConnectorId,
    generation: ConnectorGeneration,
    agent: AgentTarget,
    terminal: RuntimeTarget,
    sequence: u64,
    state: ReportedState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationAck {
    pub connector_id: ConnectorId,
    pub generation: ConnectorGeneration,
    #[serde(deserialize_with = "validation::deserialize_sequence")]
    pub sequence: u64,
    pub cursor: ObservationCursor,
    pub duplicate: bool,
}

impl Publication {
    pub fn validate(&self) -> Result<(), ConnectorError> {
        if self.agent.address.hub_id != self.terminal.address.hub_id {
            return Err(ConnectorError::InvalidRequest);
        }
        validation::sequence(self.sequence)?;
        self.state.validate()
    }
}

impl PublicationAck {
    pub fn validate(&self) -> Result<(), ConnectorError> {
        validation::sequence(self.sequence)
    }
}

impl TryFrom<PublicationWire> for Publication {
    type Error = ConnectorError;
    fn try_from(wire: PublicationWire) -> Result<Self, Self::Error> {
        let value = Self {
            connector_id: wire.connector_id,
            generation: wire.generation,
            agent: wire.agent,
            terminal: wire.terminal,
            sequence: wire.sequence,
            state: wire.state,
        };
        value.validate()?;
        Ok(value)
    }
}
