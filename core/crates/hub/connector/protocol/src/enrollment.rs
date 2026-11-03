use crate::{ConnectorCredential, ConnectorError, ConnectorScope, validation};
use serde::{Deserialize, Serialize};
use yolo_hub_identity::{AgentAddress, ConnectorId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentStatus {
    Enabled,
    Revoked,
    Superseded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EnrollmentWire")]
pub struct Enrollment {
    pub connector_id: ConnectorId,
    pub agent: AgentAddress,
    pub scope: ConnectorScope,
    pub credential: ConnectorCredential,
    pub revision: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnrollmentWire {
    connector_id: ConnectorId,
    agent: AgentAddress,
    scope: ConnectorScope,
    credential: ConnectorCredential,
    revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SummaryWire")]
pub struct EnrollmentSummary {
    pub connector_id: ConnectorId,
    pub agent: AgentAddress,
    pub scope: ConnectorScope,
    pub revision: u64,
    pub status: EnrollmentStatus,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SummaryWire {
    connector_id: ConnectorId,
    agent: AgentAddress,
    scope: ConnectorScope,
    revision: u64,
    status: EnrollmentStatus,
}

impl Enrollment {
    pub fn validate(&self) -> Result<(), ConnectorError> {
        self.scope.validate_for(self.agent.hub_id)?;
        validation::revision(self.revision)
    }
}

impl EnrollmentSummary {
    pub fn validate(&self) -> Result<(), ConnectorError> {
        self.scope.validate_for(self.agent.hub_id)?;
        validation::revision(self.revision)
    }
}

impl TryFrom<EnrollmentWire> for Enrollment {
    type Error = ConnectorError;
    fn try_from(wire: EnrollmentWire) -> Result<Self, Self::Error> {
        let value = Self {
            connector_id: wire.connector_id,
            agent: wire.agent,
            scope: wire.scope,
            credential: wire.credential,
            revision: wire.revision,
        };
        value.validate()?;
        Ok(value)
    }
}

impl TryFrom<SummaryWire> for EnrollmentSummary {
    type Error = ConnectorError;
    fn try_from(wire: SummaryWire) -> Result<Self, Self::Error> {
        let value = Self {
            connector_id: wire.connector_id,
            agent: wire.agent,
            scope: wire.scope,
            revision: wire.revision,
            status: wire.status,
        };
        value.validate()?;
        Ok(value)
    }
}
