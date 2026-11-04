use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;

use crate::{
    AGENT_PROTOCOL_SCHEMA_VERSION, PREVIOUS_AGENT_PROTOCOL_SCHEMA_VERSION, Activity, AgentCapabilities, AgentWorkingDirectory, Capability,
    Freshness, Interaction, InteractionKind, Lifecycle, ObservationCursor, ObservationSource,
    ProtocolError, ObservedProvider, Reachability, RecoveryStatus,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AgentSnapshotFields", into = "AgentSnapshotFields")]
pub struct AgentSnapshot(AgentSnapshotFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSnapshotFields {
    pub schema_version: u16,
    pub cursor: ObservationCursor,
    pub terminal: RuntimeTarget,
    pub provider: ObservedProvider,
    /// Origin metadata shares this snapshot's freshness; absent means unobserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<AgentWorkingDirectory>,
    pub lifecycle: Lifecycle,
    pub activity: Activity,
    pub reachability: Reachability,
    pub freshness: Freshness,
    pub source: ObservationSource,
    pub observed_unix_ms: u64,
    pub recovery: RecoveryStatus,
    pub capabilities: AgentCapabilities,
    pub interaction: Option<Interaction>,
}

impl AgentSnapshot {
    pub fn new(fields: AgentSnapshotFields) -> Result<Self, ProtocolError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &AgentSnapshotFields {
        &self.0
    }

    pub fn cursor(&self) -> ObservationCursor {
        self.0.cursor
    }
}

impl TryFrom<AgentSnapshotFields> for AgentSnapshot {
    type Error = ProtocolError;

    fn try_from(fields: AgentSnapshotFields) -> Result<Self, Self::Error> {
        // The previous wire schema is accepted so a persisted projection can
        // be migrated in place; new snapshots always carry the current schema.
        if fields.schema_version != AGENT_PROTOCOL_SCHEMA_VERSION
            && fields.schema_version != PREVIOUS_AGENT_PROTOCOL_SCHEMA_VERSION
        {
            return Err(ProtocolError::UnsupportedSchema);
        }
        if fields.cursor.target().address.hub_id != fields.terminal.address.hub_id {
            return Err(ProtocolError::OriginMismatch);
        }
        if matches!(fields.lifecycle, Lifecycle::Exited { .. })
            && (!matches!(fields.activity, Activity::Unknown | Activity::Idle)
                || fields.interaction.is_some()
                || fields.capabilities.interrupt == Capability::Available {}
                || fields.capabilities.submit_prompt == Capability::Available {}
                || fields.capabilities.respond_interaction == Capability::Available {})
        {
            return Err(ProtocolError::InvalidState);
        }
        if let Some(interaction) = &fields.interaction {
            if interaction.reference().target != fields.cursor.target() {
                return Err(ProtocolError::TargetMismatch);
            }
            let expected_activity = match interaction.kind() {
                InteractionKind::Approval => Activity::WaitingForApproval,
                InteractionKind::Choice | InteractionKind::Text => Activity::WaitingForInput,
            };
            if fields.activity != expected_activity {
                return Err(ProtocolError::InvalidState);
            }
        } else if fields.capabilities.respond_interaction == (Capability::Available {}) {
            return Err(ProtocolError::InvalidState);
        }
        Ok(Self(fields))
    }
}

impl From<AgentSnapshot> for AgentSnapshotFields {
    fn from(snapshot: AgentSnapshot) -> Self {
        snapshot.0
    }
}
