pub(crate) mod convert;
pub(crate) mod event;
pub(crate) mod history;
pub(crate) mod lifecycle;
pub(crate) mod reachability;
pub(crate) mod revision;
pub(crate) mod revocation;
pub(crate) mod validation;
pub(crate) mod wire;

use crate::constants::REDUCER_SNAPSHOT_SCHEMA_VERSION;
use crate::event::{EventHistory, EventIds};
use crate::identity::{ExpectedIdentity, HookProviderKind, ProviderVersion};
use crate::reducer::Reducer;
use crate::snapshot::convert::{
    binding_to_dto, identity_from_dto, provider_from_dto, version_from_dto,
};
use crate::snapshot::history::{history_from_dto, history_to_dto};
use crate::snapshot::lifecycle::{lifecycle_from_dto, lifecycle_to_dto};
use crate::snapshot::reachability::validate_reachability;
use crate::snapshot::revision::AuthorityRevision;
use crate::snapshot::validation::validate_lifecycle;
use crate::snapshot::wire::SnapshotDocument;
use crate::state::NormalizedLifecycle;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
/// A bounded witness for the reducer's canonical state.
///
/// Full retained history proves genesis reachability; a truncated tail proves
/// local invariants only. File-identity CAS prevents concurrent replacement,
/// but reboot rollback resistance requires an external monotonic anchor.
pub struct ReducerSnapshot(SnapshotDocument);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotViolation {
    InvalidExpectedIdentity,
    SchemaMismatch,
    IdentityMismatch,
    ProviderMismatch,
    VersionMismatch,
    InvalidRevision,
    HistoryTooLarge,
    InvalidHistory,
    InvalidState,
}

impl Display for SnapshotViolation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "lifecycle snapshot violation: {self:?}")
    }
}

impl Error for SnapshotViolation {}

impl Reducer {
    pub fn snapshot(&self) -> Result<ReducerSnapshot, SnapshotViolation> {
        validate_lifecycle(&self.state)?;
        validate_reachability(&self.state, self.revision)?;
        self.revision.validate_for(&self.state)?;
        Ok(ReducerSnapshot(document_from_state(
            &self.state,
            self.revision,
        )))
    }

    pub fn restore(
        snapshot: ReducerSnapshot,
        expected_identity: ExpectedIdentity,
        expected_provider: HookProviderKind,
        expected_version: ProviderVersion,
    ) -> Result<Self, SnapshotViolation> {
        expected_identity
            .validate()
            .map_err(|_| SnapshotViolation::InvalidExpectedIdentity)?;
        let document = snapshot.0;
        if document.schema_version != REDUCER_SNAPSHOT_SCHEMA_VERSION {
            return Err(SnapshotViolation::SchemaMismatch);
        }
        let identity = identity_from_dto(document.binding.identity);
        if identity != expected_identity {
            return Err(SnapshotViolation::IdentityMismatch);
        }
        let provider = provider_from_dto(document.binding.provider);
        if provider != expected_provider {
            return Err(SnapshotViolation::ProviderMismatch);
        }
        let version = version_from_dto(document.binding.provider_version);
        if version != expected_version {
            return Err(SnapshotViolation::VersionMismatch);
        }
        let revision = AuthorityRevision::restore(document.authority_revision)?;
        let mut state = lifecycle_from_dto(
            document.lifecycle,
            identity,
            provider,
            version,
            EventHistory::new(),
            EventIds::new(),
        )?;
        let (history, event_ids) = history_from_dto(&state, document.history)?;
        state.history = history;
        state.event_ids = event_ids;
        validate_lifecycle(&state)?;
        revision.validate_for(&state)?;
        validate_reachability(&state, revision)?;
        Ok(Self { state, revision })
    }
}

impl ReducerSnapshot {
    pub(crate) const fn from_document(document: SnapshotDocument) -> Self {
        Self(document)
    }
}

pub(crate) fn document_from_state(
    state: &NormalizedLifecycle,
    revision: AuthorityRevision,
) -> SnapshotDocument {
    SnapshotDocument {
        schema_version: REDUCER_SNAPSHOT_SCHEMA_VERSION,
        authority_revision: revision.get(),
        binding: binding_to_dto(state),
        lifecycle: lifecycle_to_dto(state),
        history: history_to_dto(state),
    }
}
