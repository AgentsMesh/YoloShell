use crate::{JournalError, anchor::AnchorRecord, constants::*, storage};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_agent_control_journal::Journal;
use yolo_paths::{identity::FileIdentity, namespace_lease::NamespaceLease};
use yolo_terminal_model::RuntimeRecord;

/// Holds only control history. Unavailability must not terminate the containing PTY.
pub struct ControlRuntime {
    pub(crate) namespace: NamespaceLease,
    pub(crate) anchor_path: PathBuf,
    pub(crate) record: AnchorRecord,
    pub(crate) identity: FileIdentity,
    pub(crate) journal: Journal,
    pub(crate) poisoned: AtomicBool,
}

impl ControlRuntime {
    pub fn verify(&self, current: &RuntimeRecord) -> Result<(), JournalError> {
        let result = (|| {
            if self.poisoned.load(Ordering::Acquire) {
                return Err(JournalError::Poisoned);
            }
            self.record
                .validate(current)
                .map_err(|_| JournalError::ScopeMismatch)?;
            let retained = (|| -> anyhow::Result<()> {
                self.namespace.verify()?;
                let (record, identity): (AnchorRecord, _) =
                    self.namespace.read_json_with_identity(&self.anchor_path)?;
                anyhow::ensure!(
                    record == self.record && identity == self.identity,
                    CHANGED_ANCHOR
                );
                self.namespace.verify()
            })();
            retained.map_err(|_| JournalError::Unavailable)?;
            self.journal.verify()
        })();
        self.retain_failure(result)
    }

    pub(crate) fn retain_failure<T>(
        &self,
        result: Result<T, JournalError>,
    ) -> Result<T, JournalError> {
        if result
            .as_ref()
            .err()
            .is_some_and(|error| storage::poisons(*error))
        {
            self.poisoned.store(true, Ordering::Release);
        }
        result
    }
    pub(crate) fn after_write<T>(
        &self,
        current: &RuntimeRecord,
        result: Result<T, JournalError>,
    ) -> Result<T, JournalError> {
        let value = self.retain_failure(result)?;
        if self.verify(current).is_err() {
            return self.retain_failure(Err(JournalError::OutcomeUnknown));
        }
        Ok(value)
    }
}
