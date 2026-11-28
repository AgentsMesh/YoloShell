use crate::{Registry, RuntimeAdmission, constants::REVISION_STEP};
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentCapabilities, AgentSnapshot, AgentSnapshotFields,
    Freshness, Lifecycle, ObservationCursor, ObservationSource, Reachability, RecoveryStatus,
};
use yolo_hub_connector_protocol::{Activation, ConnectorError, Publication, PublicationAck};

impl Registry {
    pub fn publish(
        &mut self,
        activation: Activation,
        publication: Publication,
        admission: RuntimeAdmission,
        now_unix_ms: u64,
    ) -> Result<PublicationAck, ConnectorError> {
        publication.validate()?;
        if publication.connector_id != activation.connector_id
            || publication.generation != activation.generation
            || publication.agent != activation.agent
            || publication.terminal != activation.terminal
        {
            return Err(ConnectorError::StaleGeneration);
        }
        let scope = self.active_scope(activation)?;
        self.reconcile_one(activation.connector_id, admission)?;
        admission.require(scope.terminal)?;
        self.active_scope(activation)?;
        let (seed, mut state) = self.entry(activation.connector_id)?;
        if publication.sequence == state.last_sequence {
            if state.last_publication.as_ref() != Some(&publication) {
                return Err(ConnectorError::Sequence);
            }
            let cursor = state
                .snapshot
                .as_ref()
                .ok_or(ConnectorError::Internal)?
                .cursor();
            return Ok(ack(&publication, cursor, true));
        }
        if state.last_sequence.checked_add(REVISION_STEP) != Some(publication.sequence) {
            return Err(ConnectorError::Sequence);
        }
        if state.source_exited && !matches!(publication.state.lifecycle, Lifecycle::Exited { .. }) {
            return Err(ConnectorError::Conflict);
        }
        let context = state
            .context
            .as_ref()
            .ok_or(ConnectorError::StaleGeneration)?;
        let cursor =
            ObservationCursor::new(publication.agent, context.stream, publication.sequence)
                .map_err(|_| ConnectorError::InvalidRequest)?;
        let snapshot = AgentSnapshot::new(AgentSnapshotFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            cursor,
            terminal: scope.terminal,
            provider: scope.provider,
            working_directory: publication.state.working_directory.clone(),
            lifecycle: publication.state.lifecycle,
            activity: publication.state.activity,
            reachability: Reachability::Reachable,
            freshness: Freshness::Current,
            source: ObservationSource::Connector,
            observed_unix_ms: now_unix_ms,
            recovery: RecoveryStatus::Unsupported,
            capabilities: AgentCapabilities::UNSUPPORTED,
            interaction: None,
        })
        .map_err(|_| ConnectorError::InvalidRequest)?;
        state.source_exited = matches!(publication.state.lifecycle, Lifecycle::Exited { .. });
        state.snapshot = Some(snapshot);
        state.last_sequence = publication.sequence;
        state.last_publication = Some(publication.clone());
        self.save(&seed, state)?;
        Ok(ack(&publication, cursor, false))
    }
}
fn ack(publication: &Publication, cursor: ObservationCursor, duplicate: bool) -> PublicationAck {
    PublicationAck {
        connector_id: publication.connector_id,
        generation: publication.generation,
        sequence: publication.sequence,
        cursor,
        duplicate,
    }
}
