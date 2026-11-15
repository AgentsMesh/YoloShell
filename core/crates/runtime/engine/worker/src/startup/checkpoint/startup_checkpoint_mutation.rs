use super::{StartupCheckpointEvidence, validation};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use uuid::Uuid;

pub(crate) struct CheckpointMutation<'a> {
    pub(crate) paths: &'a Paths,
    pub(crate) guard: yolo_paths::durable_mutation::SessionDurableMutationGuard<'a>,
}

impl<'a> CheckpointMutation<'a> {
    pub(crate) fn acquire(
        paths: &'a Paths,
        namespace: &'a NamespaceLease,
        terminal_id: Uuid,
    ) -> Result<Self> {
        let guard = yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            paths,
            namespace,
            terminal_id,
        )
        .context(super::constants::LIFECYCLE_BUSY_ERROR)?;
        Ok(Self::new(paths, guard))
    }

    pub(crate) fn acquire_after_lifecycle_lock(
        paths: &'a Paths,
        namespace: &'a NamespaceLease,
        terminal_id: Uuid,
    ) -> Result<Self> {
        let guard =
            yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_after_lifecycle_lock(
                paths, namespace, terminal_id,
            )?;
        Ok(Self::new(paths, guard))
    }

    pub(crate) fn new(
        paths: &'a Paths,
        guard: yolo_paths::durable_mutation::SessionDurableMutationGuard<'a>,
    ) -> Self {
        Self { paths, guard }
    }

    pub(crate) fn read_optional(
        &self,
    ) -> Result<Option<(StartupCheckpointEvidence, FileIdentity)>> {
        let path = self
            .paths
            .worker_startup_checkpoint(self.guard.terminal_id());
        let value = super::io::read_optional(self.guard.namespace().file(&path)?, || {
            self.guard.require_no_pending_intent()
        })?;
        if let Some((evidence, _)) = value.as_ref() {
            validation::validate(evidence)?;
            anyhow::ensure!(
                evidence.terminal_id == self.guard.terminal_id(),
                super::constants::STALE_ERROR
            );
        }
        Ok(value)
    }

    pub(crate) fn write(
        &self,
        expected: Option<FileIdentity>,
        replacement: &StartupCheckpointEvidence,
    ) -> Result<FileIdentity> {
        validation::validate(replacement)?;
        anyhow::ensure!(
            replacement.terminal_id == self.guard.terminal_id(),
            super::constants::STALE_ERROR
        );
        let path = self
            .paths
            .worker_startup_checkpoint(replacement.terminal_id);
        let entry = self.guard.namespace().file(&path)?;
        crate::paths::worker_artifacts::validate_worker_startup_checkpoint_value(
            &entry,
            replacement,
        )?;
        self.guard
            .write_json_expected_with_commit_fence(&path, expected, replacement)?;
        let (stored, identity) = self
            .read_optional()?
            .ok_or_else(|| anyhow::anyhow!(super::constants::EVIDENCE_MISSING_ERROR))?;
        validation::validate(&stored)?;
        anyhow::ensure!(stored == *replacement, super::constants::STALE_ERROR);
        Ok(identity)
    }

    pub(crate) fn require_no_pending_intent(&self) -> Result<()> {
        self.guard.require_no_pending_intent()
    }

    pub(crate) fn read_optional_managed<T>(
        &self,
        path: &std::path::Path,
    ) -> Result<Option<(T, FileIdentity)>>
    where
        T: DeserializeOwned,
    {
        self.guard.read_optional_with_identity(path)
    }

    pub(crate) fn read_optional_managed_with_versions<T: DeserializeOwned>(
        &self,
        path: &std::path::Path,
    ) -> Result<Option<(T, FileIdentity, crate::paths::identity::DurableFileVersion)>> {
        self.guard.read_optional_with_versions(path)
    }
}
