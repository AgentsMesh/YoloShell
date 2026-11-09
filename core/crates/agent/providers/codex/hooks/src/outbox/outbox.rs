use super::constants::{FIRST_SEQUENCE, OUTBOX_SCHEMA_VERSION, errors};
use super::lock::OutboxLock;
use super::{outbox_storage, outbox_validation};
use crate::agent::hooks::{HookEnvelope, HookIdentity, ZERO_SEQUENCE};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexOutboxState {
    pub(crate) schema_version: u16,
    pub(crate) acknowledged_sequence: u64,
    pub(crate) next_sequence: u64,
    pub(crate) pending: VecDeque<HookEnvelope>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(crate) owner: Option<HookIdentity>,
    pub(crate) acknowledged_events: VecDeque<HookEnvelope>,
    pub(crate) pending_occurrence_witnesses:
        VecDeque<super::outbox_pending_witness::CodexPendingOccurrenceWitness>,
    pub(crate) occurrence_receipts: VecDeque<super::outbox_receipt::CodexOccurrenceReceipt>,
    pub(crate) durable_rejections: VecDeque<super::outbox_rejection::CodexDurableRejection>,
}

include!("outbox_state_access.rs");

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/outbox/outbox_current_shape_tests.rs"]
mod current_shape_tests;

pub struct CodexAuthoritativeOccurrence {
    event: HookEnvelope,
    durable_rejection: Option<super::outbox_rejection::CodexDurableRejection>,
}

impl CodexAuthoritativeOccurrence {
    pub fn into_parts(
        self,
    ) -> (
        HookEnvelope,
        Option<super::outbox_rejection::CodexDurableRejection>,
    ) {
        (self.event, self.durable_rejection)
    }
}

impl Default for CodexOutboxState {
    fn default() -> Self {
        Self {
            schema_version: OUTBOX_SCHEMA_VERSION,
            acknowledged_sequence: ZERO_SEQUENCE,
            next_sequence: FIRST_SEQUENCE,
            pending: VecDeque::new(),
            owner: None,
            acknowledged_events: VecDeque::new(),
            pending_occurrence_witnesses: VecDeque::new(),
            occurrence_receipts: VecDeque::new(),
            durable_rejections: VecDeque::new(),
        }
    }
}

#[derive(Debug)]
pub struct CodexOutbox {
    pub(super) directory: crate::paths::directory::PrivateDirectory,
    pub(super) path: PathBuf,
    pub(super) owner: Option<HookIdentity>,
}

impl CodexOutbox {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Self::with_owner(path.into(), None)
    }

    pub fn for_identity(path: impl Into<PathBuf>, identity: &HookIdentity) -> Result<Self> {
        identity
            .validate()
            .map_err(|error| anyhow::anyhow!(error))?;
        Self::with_owner(path.into(), Some(identity.clone()))
    }

    fn with_owner(path: PathBuf, owner: Option<HookIdentity>) -> Result<Self> {
        super::lock::validate_path(&path)?;
        let directory =
            crate::paths::secure_private_parent(&path).context(errors::OUTBOX_CORRUPT)?;
        let outbox = Self {
            directory,
            path,
            owner,
        };
        let lock = OutboxLock::acquire(&outbox.directory, &outbox.path)?;
        let loaded = outbox_storage::load(&lock, &outbox.path)?;
        outbox_validation::check_configured(&loaded.state, outbox.owner.as_ref())?;
        Ok(outbox)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<CodexOutboxState> {
        let _lock = OutboxLock::acquire(&self.directory, &self.path)?;
        let loaded = outbox_storage::load(&_lock, &self.path)?;
        outbox_validation::check_configured(&loaded.state, self.owner.as_ref())?;
        Ok(loaded.state)
    }

    pub fn with_locked_state<T>(
        &self,
        operation: impl FnOnce(&CodexOutboxState) -> Result<T>,
    ) -> Result<T> {
        let lock = OutboxLock::acquire(&self.directory, &self.path)?;
        let loaded = outbox_storage::load(&lock, &self.path)?;
        outbox_validation::check_configured(&loaded.state, self.owner.as_ref())?;
        let result = operation(&loaded.state);
        outbox_storage::run_after_operation_hook();
        outbox_storage::verify_unchanged(&lock, &self.path, &loaded)?;
        result
    }

    pub fn pending(&self) -> Result<Vec<HookEnvelope>> {
        Ok(self.load()?.pending.into_iter().collect())
    }

    pub fn authoritative_intake_occurrence(
        &self,
        occurrence: &super::CodexIntakeOccurrence,
    ) -> Result<Option<HookEnvelope>> {
        Ok(self
            .authoritative_intake_outcome(occurrence)?
            .map(|outcome| outcome.event))
    }

    pub fn authoritative_intake_outcome(
        &self,
        occurrence: &super::CodexIntakeOccurrence,
    ) -> Result<Option<CodexAuthoritativeOccurrence>> {
        let identity = occurrence.durable_identity()?;
        self.authoritative_occurrence_inner(occurrence.envelope(), Some(&identity))
    }

    #[cfg(test)]
    pub(crate) fn authoritative_occurrence(
        &self,
        occurrence: &HookEnvelope,
    ) -> Result<Option<HookEnvelope>> {
        Ok(self
            .authoritative_occurrence_inner(occurrence, None)?
            .map(|outcome| outcome.event))
    }

    fn authoritative_occurrence_inner(
        &self,
        occurrence: &HookEnvelope,
        identity: Option<&super::CodexOccurrenceIdentity>,
    ) -> Result<Option<CodexAuthoritativeOccurrence>> {
        ensure!(
            !occurrence.event_id.is_nil() && occurrence.sequence.0 == ZERO_SEQUENCE,
            errors::OUTBOX_CORRUPT
        );
        self.with_locked_state(|state| {
            let Some(event) = outbox_validation::find_explicit_replay(state, occurrence, identity)?
            else {
                return Ok(None);
            };
            let durable_rejection = state.durable_rejection_for(&event)?;
            Ok(Some(CodexAuthoritativeOccurrence {
                event,
                durable_rejection,
            }))
        })
    }

    pub fn drain<F>(&self, mut deliver: F) -> Result<()>
    where
        F: FnMut(&HookEnvelope) -> Result<()>,
    {
        loop {
            let event = {
                // Hold the lock only while selecting the durable head. Delivery can block.
                let lock = OutboxLock::acquire(&self.directory, &self.path)?;
                let loaded = outbox_storage::load(&lock, &self.path)?;
                outbox_validation::check_configured(&loaded.state, self.owner.as_ref())?;
                loaded.state.pending.front().cloned()
            };
            let Some(event) = event else {
                return Ok(());
            };
            deliver(&event)?;
            self.acknowledge(event.sequence.0, event.event_id)?;
        }
    }
}
