use crate::{ConnectorError, validation};
use serde::{Deserialize, Serialize};
use yolo_hub_identity::{AgentTarget, ConnectorGeneration, ConnectorId, RuntimeTarget};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ActivationWire")]
pub struct Activation {
    pub connector_id: ConnectorId,
    pub generation: ConnectorGeneration,
    pub agent: AgentTarget,
    pub terminal: RuntimeTarget,
    pub next_sequence: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivationWire {
    connector_id: ConnectorId,
    generation: ConnectorGeneration,
    agent: AgentTarget,
    terminal: RuntimeTarget,
    next_sequence: u64,
}

impl Activation {
    pub fn validate(&self) -> Result<(), ConnectorError> {
        if self.agent.address.hub_id != self.terminal.address.hub_id {
            return Err(ConnectorError::InvalidRequest);
        }
        validation::sequence(self.next_sequence)
    }
}

impl TryFrom<ActivationWire> for Activation {
    type Error = ConnectorError;
    fn try_from(wire: ActivationWire) -> Result<Self, Self::Error> {
        let value = Self {
            connector_id: wire.connector_id,
            generation: wire.generation,
            agent: wire.agent,
            terminal: wire.terminal,
            next_sequence: wire.next_sequence,
        };
        value.validate()?;
        Ok(value)
    }
}
