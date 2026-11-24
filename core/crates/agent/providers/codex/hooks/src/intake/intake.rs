use super::constants::errors;
use super::intake_record::{CodexIntakeQuarantine, CodexIntakeRecord};
use anyhow::{Context, Result, ensure};
use std::ffi::OsStr;
use std::path::PathBuf;

#[path = "intake_completion.rs"]
mod completion;

#[derive(Debug)]
pub struct CodexHookIntake {
    directory: crate::paths::directory::PrivateDirectory,
    invocation: crate::paths::directory::PrivateDirectory,
    owner: crate::agent::hooks::HookIdentity,
}

#[derive(Debug)]
pub struct CodexIntakeOccurrence {
    file_name: String,
    identity: crate::paths::identity::FileIdentity,
    record: CodexIntakeRecord,
}

#[derive(Debug, Default)]
pub struct CodexIntakeScan {
    pub occurrences: Vec<CodexIntakeOccurrence>,
    pub quarantined: usize,
    pub deferred: usize,
}

impl CodexHookIntake {
    pub fn for_identity(
        directory: impl Into<PathBuf>,
        owner: &crate::agent::hooks::HookIdentity,
    ) -> Result<Self> {
        owner
            .validate()
            .map_err(|_| anyhow::anyhow!(errors::INTAKE_CORRUPT))?;
        let path = directory.into();
        ensure!(
            path.file_name() == Some(OsStr::new(super::intake_constants::DIRECTORY_NAME)),
            errors::INTAKE_CORRUPT
        );
        let parent = path.parent().context(errors::INTAKE_CORRUPT)?;
        let invocation = crate::paths::directory::PrivateDirectory::ensure(parent)
            .context(errors::INTAKE_STORAGE)?;
        let directory = invocation
            .ensure_descendant(&path)
            .context(errors::INTAKE_STORAGE)?;
        Ok(Self {
            directory,
            invocation,
            owner: owner.clone(),
        })
    }

    pub(super) fn open_for_invocation(
        invocation: &crate::paths::directory::PrivateDirectory,
        owner: &crate::agent::hooks::HookIdentity,
    ) -> Result<Self> {
        owner
            .validate()
            .map_err(|_| anyhow::anyhow!(errors::INTAKE_CORRUPT))?;
        let invocation = invocation.try_clone().context(errors::INTAKE_STORAGE)?;
        let path = invocation
            .path()
            .join(super::intake_constants::DIRECTORY_NAME);
        let directory = invocation
            .open_descendant(&path)
            .context(errors::INTAKE_STORAGE)?;
        Ok(Self {
            directory,
            invocation,
            owner: owner.clone(),
        })
    }

    pub fn quarantine(&self, occurrence: &CodexIntakeOccurrence) -> Result<()> {
        self.write_quarantine(&occurrence.file_name, occurrence.identity)
    }

    pub fn verify_current(&self, occurrence: &CodexIntakeOccurrence) -> Result<()> {
        let path = self.path(&occurrence.file_name);
        let (bytes, identity) = super::intake_storage::read_bytes(&self.directory, &path)?;
        ensure!(identity == occurrence.identity, errors::INTAKE_STORAGE);
        let record = CodexIntakeRecord::decode(&bytes, &occurrence.file_name, &self.owner)?;
        ensure!(record == occurrence.record, errors::INTAKE_CORRUPT);
        Ok(())
    }

    pub(super) fn write_quarantine(
        &self,
        pending_file: &str,
        pending_identity: crate::paths::identity::FileIdentity,
    ) -> Result<()> {
        let marker = CodexIntakeQuarantine::new(pending_file.to_owned(), pending_identity);
        self.verify_pending_identity(pending_file, pending_identity)?;
        self.verify_artifact_capacity(false)?;
        super::intake_storage::create_with_fence(
            &self.directory,
            &self.path(&super::intake_record::quarantine_file_name(pending_file)),
            &marker,
            || {
                self.verify_pending_identity(pending_file, pending_identity)?;
                self.verify_artifact_capacity(true)
            },
        )?;
        self.verify_pending_identity(pending_file, pending_identity)
    }

    pub(super) fn path(&self, file_name: &str) -> PathBuf {
        self.directory.path().join(file_name)
    }

    pub(super) fn directory(&self) -> &crate::paths::directory::PrivateDirectory {
        &self.directory
    }

    pub(super) fn invocation(&self) -> &crate::paths::directory::PrivateDirectory {
        &self.invocation
    }

    pub fn owner(&self) -> &crate::agent::hooks::HookIdentity {
        &self.owner
    }

    pub(super) fn verify_pending_identity(
        &self,
        pending_file: &str,
        expected: crate::paths::identity::FileIdentity,
    ) -> Result<()> {
        let (_, observed) =
            super::intake_storage::read_bytes(&self.directory, &self.path(pending_file))?;
        ensure!(observed == expected, errors::INTAKE_STORAGE);
        Ok(())
    }
}

impl CodexIntakeOccurrence {
    pub fn envelope(&self) -> &crate::agent::hooks::HookEnvelope {
        self.record.envelope()
    }

    pub(super) fn new(
        file_name: String,
        identity: crate::paths::identity::FileIdentity,
        record: CodexIntakeRecord,
    ) -> Self {
        Self {
            file_name,
            identity,
            record,
        }
    }

    pub(crate) fn ordering_key(&self) -> (u64, uuid::Uuid) {
        self.record.ordering_key()
    }

    pub fn durable_identity(&self) -> Result<super::CodexOccurrenceIdentity> {
        super::CodexOccurrenceIdentity::capture(
            &self.file_name,
            self.identity,
            self.record.envelope(),
        )
    }
}

impl CodexIntakeQuarantine {
    pub(super) fn decode(
        bytes: &[u8],
        pending_file: &str,
        pending_identity: crate::paths::identity::FileIdentity,
    ) -> Result<Self> {
        let marker: Self = serde_json::from_slice(bytes).context(errors::INTAKE_CORRUPT)?;
        marker.validate(pending_file, pending_identity)?;
        Ok(marker)
    }
}

impl CodexIntakeRecord {
    pub(super) fn decode(
        bytes: &[u8],
        file_name: &str,
        owner: &crate::agent::hooks::HookIdentity,
    ) -> Result<Self> {
        let record: Self = serde_json::from_slice(bytes).context(errors::INTAKE_CORRUPT)?;
        record.validate(file_name, owner)?;
        Ok(record)
    }
}
