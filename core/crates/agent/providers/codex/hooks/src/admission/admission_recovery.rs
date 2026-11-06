use super::admission_storage::{CODEX_ADMISSION_FILE_NAME, scope_matches};
use super::{
    CodexAdmissionArtifact, CodexAdmissionAuthority, CodexAdmissionRecord, CodexAdmissionStorage,
    CodexHookArtifacts,
};
use crate::agent::hooks::Generation;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result, ensure};
use std::sync::Arc;

const RECOVERY_CONFLICT: &str = "Codex admission recovery conflicts with durable authority";

impl CodexAdmissionAuthority {
    pub fn recover_artifacts(artifacts: &CodexHookArtifacts) -> Result<Option<Self>> {
        let directory = Arc::new(
            crate::paths::directory::PrivateDirectory::open(artifacts.directory())
                .context(RECOVERY_CONFLICT)?,
        );
        let path = directory.path().join(CODEX_ADMISSION_FILE_NAME);
        let entry = PrivateFileEntry::from_directory(directory.try_clone()?, &path)?;
        if entry.is_missing()? {
            directory.verify_path().context(RECOVERY_CONFLICT)?;
            return Ok(None);
        }
        let (record, file_identity): (CodexAdmissionRecord, _) =
            crate::paths::read_entry_guarded_with_identity(entry, || directory.verify_path())?;
        let owner = record.request().identity.clone();
        owner.validate().map_err(anyhow::Error::new)?;
        ensure!(owner.generation == Generation::INITIAL, RECOVERY_CONFLICT);
        ensure!(scope_matches(directory.path(), &owner), RECOVERY_CONFLICT);
        let storage = CodexAdmissionStorage {
            directory,
            path,
            owner,
        };
        let artifact = CodexAdmissionArtifact {
            record,
            file_identity,
        };
        let authority = Self::from_recovered(storage, artifact)?;
        authority.verify().context(RECOVERY_CONFLICT)?;
        Ok(Some(authority))
    }

    fn from_recovered(
        storage: CodexAdmissionStorage,
        artifact: CodexAdmissionArtifact,
    ) -> Result<Self> {
        let authority = Self::from_artifact(storage, artifact)?;
        authority.verify()?;
        Ok(authority)
    }
}
