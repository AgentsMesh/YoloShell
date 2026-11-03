use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AgentCapabilities, AgentSnapshot, Freshness, Lifecycle, ObservationSource, Reachability,
    RecoveryStatus,
};
use yolo_hub_connector_protocol::{EnrollmentStatus, FIRST_CONNECTOR_REVISION, Publication};
use yolo_hub_connector_store::{Seed, StoredState};
use yolo_hub_identity::{ConnectorGeneration, ConnectorInstanceId, ExecutionId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    pub instance: ConnectorInstanceId,
    pub execution: ExecutionId,
    pub generation: ConnectorGeneration,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub stream: Uuid,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Revocation {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    pub expected_revision: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    pub schema: u16,
    pub revision: u64,
    pub status: EnrollmentStatus,
    pub context: Option<Context>,
    pub last_sequence: u64,
    pub last_publication: Option<Publication>,
    pub snapshot: Option<AgentSnapshot>,
    pub source_exited: bool,
    pub revocation: Option<Revocation>,
}
impl State {
    pub fn initial() -> Self {
        Self {
            schema: STATE_SCHEMA,
            revision: FIRST_CONNECTOR_REVISION,
            status: EnrollmentStatus::Enabled,
            context: None,
            last_sequence: INITIAL_SEQUENCE,
            last_publication: None,
            snapshot: None,
            source_exited: false,
            revocation: None,
        }
    }
}
impl StoredState for State {
    fn validate(&self, seed: &Seed) -> Result<()> {
        ensure!(
            self.schema == STATE_SCHEMA && self.revision >= FIRST_CONNECTOR_REVISION,
            INVALID_STATE
        );
        if let Some(revocation) = &self.revocation {
            ensure!(
                self.status == EnrollmentStatus::Revoked
                    && !revocation.request_id.is_nil()
                    && revocation.expected_revision >= FIRST_CONNECTOR_REVISION
                    && revocation.expected_revision.checked_add(REVISION_STEP)
                        == Some(self.revision),
                INVALID_STATE
            );
        } else {
            ensure!(self.status != EnrollmentStatus::Revoked, INVALID_STATE);
        }
        let Some(context) = &self.context else {
            ensure!(
                self.last_sequence == INITIAL_SEQUENCE
                    && self.last_publication.is_none()
                    && self.snapshot.is_none()
                    && !self.source_exited,
                INVALID_STATE
            );
            return Ok(());
        };
        ensure!(!context.stream.is_nil(), INVALID_STATE);
        if let Some(snapshot) = &self.snapshot {
            let fields = snapshot.fields();
            ensure!(
                fields.cursor.target().address == seed.agent
                    && fields.terminal == seed.scope.terminal
                    && fields.provider == seed.scope.provider
                    && fields.source == ObservationSource::Connector
                    && fields.recovery == RecoveryStatus::Unsupported
                    && fields.capabilities == AgentCapabilities::UNSUPPORTED
                    && fields.interaction.is_none()
                    && ((fields.freshness == Freshness::Current
                        && fields.reachability == Reachability::Reachable)
                        || (fields.freshness == Freshness::Stale
                            && fields.reachability == Reachability::Unreachable)),
                INVALID_SNAPSHOT
            );
            if self.status != EnrollmentStatus::Enabled {
                ensure!(
                    fields.freshness == Freshness::Stale
                        && fields.reachability == Reachability::Unreachable,
                    INVALID_STATE
                );
            }
            if self.source_exited {
                ensure!(
                    fields.cursor.target().execution_id == context.execution
                        && matches!(fields.lifecycle, Lifecycle::Exited { .. }),
                    INVALID_STATE
                );
            }
        } else {
            ensure!(!self.source_exited, INVALID_STATE);
        }
        if let Some(publication) = &self.last_publication {
            publication.validate()?;
            ensure!(
                publication.connector_id == seed.connector_id
                    && publication.generation == context.generation
                    && publication.agent.address == seed.agent
                    && publication.agent.execution_id == context.execution
                    && publication.terminal == seed.scope.terminal
                    && publication.sequence == self.last_sequence,
                INVALID_PUBLICATION
            );
            let fields = self
                .snapshot
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!(INVALID_STATE))?
                .fields();
            ensure!(
                fields.cursor.target() == publication.agent
                    && fields.cursor.stream() == context.stream
                    && (if fields.freshness == Freshness::Current {
                        fields.cursor.sequence() == publication.sequence
                    } else {
                        publication.sequence.checked_add(REVISION_STEP)
                            == Some(fields.cursor.sequence())
                    })
                    && fields.lifecycle == publication.state.lifecycle
                    && fields.activity == publication.state.activity
                    && fields.working_directory == publication.state.working_directory,
                INVALID_PUBLICATION
            );
            ensure!(
                self.source_exited == matches!(fields.lifecycle, Lifecycle::Exited { .. }),
                INVALID_STATE
            );
        } else {
            ensure!(self.last_sequence == INITIAL_SEQUENCE, INVALID_STATE);
            ensure!(
                self.snapshot.as_ref().is_none_or(|snapshot| {
                    snapshot.fields().freshness == Freshness::Stale
                        && snapshot.fields().reachability == Reachability::Unreachable
                }),
                INVALID_STATE
            );
        }
        Ok(())
    }
}
