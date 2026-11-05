use super::{CodexAdmissionRecord, CodexHookArtifacts, CodexInvocationAuthority};
use crate::agent::hooks::{Generation, HookIdentity};
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result, ensure};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[path = "admission_storage_publish.rs"]
mod publish;

pub(crate) const CODEX_ADMISSION_FILE_NAME: &str = "admission.json";
const ADMISSION_CONFLICT: &str = "Codex invocation admission conflicts with durable authority";
const ADMISSION_SCOPE: &str = "Codex invocation admission artifact scope is invalid";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexAdmissionArtifact {
    pub(super) record: CodexAdmissionRecord,
    pub(super) file_identity: FileIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexAdmissionPublication {
    artifact: CodexAdmissionArtifact,
    replay: bool,
}

#[derive(Clone, Debug)]
pub struct CodexAdmissionStorage {
    pub(super) directory: Arc<crate::paths::directory::PrivateDirectory>,
    pub(super) path: PathBuf,
    pub(super) owner: HookIdentity,
}

impl CodexAdmissionStorage {
    pub fn for_identity(artifacts: &CodexHookArtifacts, owner: &HookIdentity) -> Result<Self> {
        owner
            .validate()
            .map_err(|_| anyhow::anyhow!(ADMISSION_SCOPE))?;
        ensure!(owner.generation == Generation::INITIAL, ADMISSION_SCOPE);
        ensure!(scope_matches(artifacts.directory(), owner), ADMISSION_SCOPE);
        let directory = Arc::new(
            crate::paths::directory::PrivateDirectory::ensure(artifacts.directory())
                .context(ADMISSION_CONFLICT)?,
        );
        Ok(Self {
            path: directory.path().join(CODEX_ADMISSION_FILE_NAME),
            directory,
            owner: owner.clone(),
        })
    }

    pub fn read(&self) -> Result<Option<CodexAdmissionArtifact>> {
        self.directory.verify_path().context(ADMISSION_CONFLICT)?;
        let entry = self.entry()?;
        if entry.is_missing()? {
            self.directory.verify_path().context(ADMISSION_CONFLICT)?;
            return Ok(None);
        }
        let (record, file_identity) =
            crate::paths::read_entry_guarded_with_identity(entry, || self.directory.verify_path())?;
        let artifact = CodexAdmissionArtifact {
            record,
            file_identity,
        };
        artifact.record.validate_for(&self.owner)?;
        Ok(Some(artifact))
    }

    pub(crate) fn verify(&self, expected: &CodexAdmissionArtifact) -> Result<()> {
        ensure!(self.read()?.as_ref() == Some(expected), ADMISSION_CONFLICT);
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn invocation_authority(&self) -> Result<CodexInvocationAuthority> {
        CodexInvocationAuthority::capture(&self.directory).context(ADMISSION_CONFLICT)
    }

    pub(super) fn directory(&self) -> &crate::paths::directory::PrivateDirectory {
        self.directory.as_ref()
    }

    pub(super) fn owner(&self) -> &HookIdentity {
        &self.owner
    }

    pub(super) fn entry(&self) -> Result<PrivateFileEntry> {
        PrivateFileEntry::from_directory(self.directory.try_clone()?, &self.path)
            .context(ADMISSION_CONFLICT)
    }
}

impl CodexAdmissionArtifact {
    pub fn record(&self) -> &CodexAdmissionRecord {
        &self.record
    }

    pub(crate) fn file_identity(&self) -> FileIdentity {
        self.file_identity
    }
}

impl CodexAdmissionPublication {
    fn new(artifact: CodexAdmissionArtifact, replay: bool) -> Self {
        Self { artifact, replay }
    }

    pub(crate) fn artifact(&self) -> &CodexAdmissionArtifact {
        &self.artifact
    }

    pub(crate) fn is_replay(&self) -> bool {
        self.replay
    }

    pub(crate) fn into_artifact(self) -> CodexAdmissionArtifact {
        self.artifact
    }
}

pub(super) fn scope_matches(directory: &Path, owner: &HookIdentity) -> bool {
    let Some(runtime) = directory.parent() else {
        return false;
    };
    let Some(root) = runtime.parent() else {
        return false;
    };
    let Some(session) = root.parent() else {
        return false;
    };
    directory.file_name() == Some(OsStr::new(&owner.invocation_id.to_string()))
        && runtime.file_name() == Some(OsStr::new(&owner.runtime_epoch.to_string()))
        && root.file_name() == Some(OsStr::new(super::constants::ARTIFACTS_DIRECTORY_NAME))
        && session.file_name() == Some(OsStr::new(&owner.terminal_id.to_string()))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/admission/admission_storage_adversarial_tests.rs"]
mod adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/admission/admission_storage_executable_tests.rs"]
mod executable_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/admission/admission_storage_tests.rs"]
mod tests;
