use crate::{
    Registry, RuntimeAdmission,
    constants::REVISION_STEP,
    state::{Revocation, State},
};
use uuid::Uuid;
use yolo_hub_connector_protocol::{
    ConnectorCredential, ConnectorError, ConnectorScope, Enrollment, EnrollmentStatus,
    EnrollmentSummary, MAX_CONNECTORS,
};
use yolo_hub_connector_store::Seed;
use yolo_hub_identity::{AgentAddress, AgentId, ConnectorId};

impl Registry {
    pub fn register(
        &mut self,
        request_id: Uuid,
        scope: ConnectorScope,
        admission: RuntimeAdmission,
    ) -> Result<Enrollment, ConnectorError> {
        if request_id.is_nil() {
            return Err(ConnectorError::InvalidRequest);
        }
        scope.validate_for(self.identity.hub())?;
        admission.require(scope.terminal)?;
        let entries = self.entries()?;
        if let Some((seed, state)) = entries
            .iter()
            .find(|(seed, _)| seed.request_id == request_id)
        {
            if seed.scope != scope || state.status != EnrollmentStatus::Enabled {
                return Err(ConnectorError::Conflict);
            }
            return Ok(enrollment(seed, state));
        }
        if entries.len() >= MAX_CONNECTORS {
            return Err(ConnectorError::Capacity);
        }
        if entries.iter().any(|(seed, state)| {
            seed.scope.terminal == scope.terminal && state.status == EnrollmentStatus::Enabled
        }) {
            return Err(ConnectorError::Conflict);
        }
        let seed = Seed {
            connector_id: ConnectorId::new(),
            request_id,
            agent: AgentAddress {
                hub_id: self.identity.hub(),
                agent_id: AgentId::new(),
            },
            scope,
            credential: ConnectorCredential::generate(),
        };
        let state = State::initial();
        if self.store.insert(seed.clone(), state.clone()).is_err() {
            self.live.clear();
            return Err(ConnectorError::OutcomeUnknown);
        }
        self.cache
            .insert(seed.connector_id, (seed.clone(), state.clone()));
        Ok(enrollment(&seed, &state))
    }
    pub fn list(&self) -> Result<Vec<EnrollmentSummary>, ConnectorError> {
        Ok(self
            .entries()?
            .iter()
            .map(|(seed, state)| summary(seed, state))
            .collect())
    }
    pub fn revoke(
        &mut self,
        request_id: Uuid,
        id: ConnectorId,
        expected_revision: u64,
    ) -> Result<EnrollmentSummary, ConnectorError> {
        if request_id.is_nil() {
            return Err(ConnectorError::InvalidRequest);
        }
        let (seed, mut state) = self.entry(id)?;
        let revocation = Revocation {
            request_id,
            expected_revision,
        };
        if state.revocation.as_ref() == Some(&revocation) {
            return Ok(summary(&seed, &state));
        }
        if state.revision != expected_revision || state.status == EnrollmentStatus::Revoked {
            return Err(ConnectorError::Conflict);
        }
        state.revision = state
            .revision
            .checked_add(REVISION_STEP)
            .ok_or(ConnectorError::Capacity)?;
        state.status = EnrollmentStatus::Revoked;
        state.revocation = Some(revocation);
        crate::observations::stale(&mut state)?;
        self.save(&seed, state.clone())?;
        self.live.remove(&id);
        Ok(summary(&seed, &state))
    }
}
pub(crate) fn summary(seed: &Seed, state: &State) -> EnrollmentSummary {
    EnrollmentSummary {
        connector_id: seed.connector_id,
        agent: seed.agent,
        scope: seed.scope.clone(),
        revision: state.revision,
        status: state.status,
    }
}
fn enrollment(seed: &Seed, state: &State) -> Enrollment {
    Enrollment {
        connector_id: seed.connector_id,
        agent: seed.agent,
        scope: seed.scope.clone(),
        credential: seed.credential,
        revision: state.revision,
    }
}
