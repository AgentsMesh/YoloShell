use crate::{Registry, RuntimeAdmission, constants::INITIAL_SEQUENCE, state::Context};
use uuid::Uuid;
use yolo_hub_connector_protocol::{
    Activation, ConnectorCredential, ConnectorError, ConnectorScope, EnrollmentStatus,
    FIRST_PUBLICATION_SEQUENCE,
};
use yolo_hub_identity::{
    AgentTarget, ConnectorGeneration, ConnectorId, ConnectorInstanceId, ExecutionId,
};

impl Registry {
    pub fn credential_scope(
        &self,
        id: ConnectorId,
        credential: ConnectorCredential,
    ) -> Result<ConnectorScope, ConnectorError> {
        let (seed, state) = self.entry(id)?;
        if !seed.credential.matches(&credential) || state.status != EnrollmentStatus::Enabled {
            return Err(ConnectorError::Unauthorized);
        }
        Ok(seed.scope)
    }
    pub fn activate(
        &mut self,
        id: ConnectorId,
        credential: ConnectorCredential,
        instance: ConnectorInstanceId,
        admission: RuntimeAdmission,
    ) -> Result<Activation, ConnectorError> {
        let scope = self.credential_scope(id, credential)?;
        self.reconcile_one(id, admission)?;
        admission.require(scope.terminal)?;
        let (seed, mut state) = self.entry(id)?;
        if state.status != EnrollmentStatus::Enabled {
            return Err(ConnectorError::Unauthorized);
        }
        let same_instance = state
            .context
            .as_ref()
            .is_some_and(|old| old.instance == instance);
        let execution = state
            .context
            .as_ref()
            .filter(|_| same_instance)
            .map(|old| old.execution)
            .unwrap_or_else(ExecutionId::new);
        let context = Context {
            instance,
            execution,
            generation: ConnectorGeneration::new(),
            stream: Uuid::new_v4(),
        };
        let activation = Activation {
            connector_id: id,
            generation: context.generation,
            agent: AgentTarget {
                address: seed.agent,
                execution_id: execution,
            },
            terminal: scope.terminal,
            next_sequence: FIRST_PUBLICATION_SEQUENCE,
        };
        crate::observations::stale(&mut state)?;
        state.context = Some(context);
        state.last_sequence = INITIAL_SEQUENCE;
        state.last_publication = None;
        if !same_instance {
            state.source_exited = false;
        }
        self.save(&seed, state)?;
        self.live.insert(id, activation.generation);
        Ok(activation)
    }
    pub fn active_scope(&self, activation: Activation) -> Result<ConnectorScope, ConnectorError> {
        activation.validate()?;
        let (seed, state) = self.entry(activation.connector_id)?;
        let context = state.context.ok_or(ConnectorError::StaleGeneration)?;
        if state.status != EnrollmentStatus::Enabled
            || self.live.get(&seed.connector_id) != Some(&activation.generation)
            || context.generation != activation.generation
            || context.execution != activation.agent.execution_id
            || activation.agent.address != seed.agent
            || activation.terminal != seed.scope.terminal
        {
            return Err(ConnectorError::StaleGeneration);
        }
        Ok(seed.scope)
    }
}
