use super::retirement_constants::{
    CORRUPT, FILE_NAME, INTAKE_RETIRED, SEAL_FILE_NAME, STORAGE, WRITER_LOCK_FILE_NAME,
};
use super::{
    CodexAdmissionStorage, CodexHookArtifacts, CodexInvocationAuthority, CodexRetirementArtifact,
    CodexRetirementPublication, CodexRetirementRecord, CodexRetirementSeal,
    CodexRetirementSealArtifact, CodexRetirementSealPublication,
};
use crate::agent::hooks::HookIdentity;
use anyhow::{Context, Result, ensure};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[path = "retirement_storage_io.rs"]
mod storage_io;

pub(super) struct CodexRetirementStorage {
    directory: crate::paths::directory::PrivateDirectory,
    owner: HookIdentity,
    seal_path: PathBuf,
    retirement_path: PathBuf,
    writer_lock_path: PathBuf,
}

impl CodexRetirementStorage {
    pub(super) fn for_identity(
        artifacts: &CodexHookArtifacts,
        owner: &HookIdentity,
    ) -> Result<Self> {
        Self::for_directory(artifacts.directory(), owner)
    }

    pub(super) fn for_admission(admission: &CodexAdmissionStorage) -> Result<Self> {
        Self::from_directory(admission.directory().try_clone()?, admission.owner())
    }

    pub(super) fn for_intake(
        invocation: &crate::paths::directory::PrivateDirectory,
        intake_directory: &Path,
        owner: &HookIdentity,
    ) -> Result<Self> {
        ensure!(
            intake_directory.file_name()
                == Some(OsStr::new(super::intake_constants::DIRECTORY_NAME)),
            CORRUPT
        );
        ensure!(
            intake_directory.parent() == Some(invocation.path()),
            CORRUPT
        );
        Self::from_directory(invocation.try_clone()?, owner)
    }

    fn for_directory(directory: &Path, owner: &HookIdentity) -> Result<Self> {
        let directory =
            crate::paths::directory::PrivateDirectory::ensure(directory).context(STORAGE)?;
        Self::from_directory(directory, owner)
    }

    fn from_directory(
        directory: crate::paths::directory::PrivateDirectory,
        owner: &HookIdentity,
    ) -> Result<Self> {
        owner.validate().map_err(|_| anyhow::anyhow!(CORRUPT))?;
        ensure!(scope_matches(directory.path(), owner), CORRUPT);
        Ok(Self {
            seal_path: directory.path().join(SEAL_FILE_NAME),
            retirement_path: directory.path().join(FILE_NAME),
            writer_lock_path: directory.path().join(WRITER_LOCK_FILE_NAME),
            directory,
            owner: owner.clone(),
        })
    }

    pub(super) fn ensure_unsealed(&self) -> Result<()> {
        let sealed = self.inspect(&self.seal_path)?.is_some();
        let retired = self.inspect(&self.retirement_path)?.is_some();
        ensure!(!sealed && !retired, INTAKE_RETIRED);
        Ok(())
    }

    pub(super) fn publish_seal(
        &self,
        expected: &CodexRetirementSeal,
        commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<CodexRetirementSealPublication> {
        ensure!(expected.identity() == &self.owner, CORRUPT);
        let (record, file_identity, replay) =
            self.publish(&self.seal_path, expected, commit_fence)?;
        Ok(CodexRetirementSealPublication::new(
            record,
            file_identity,
            replay,
        ))
    }

    pub(super) fn publish_retirement(
        &self,
        expected: &CodexRetirementRecord,
        commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<CodexRetirementPublication> {
        ensure!(expected.seal().identity() == &self.owner, CORRUPT);
        let (record, file_identity, replay) =
            self.publish(&self.retirement_path, expected, commit_fence)?;
        Ok(CodexRetirementPublication::new(
            record,
            file_identity,
            replay,
        ))
    }

    pub(super) fn verify_seal(&self, expected: &CodexRetirementSealArtifact) -> Result<()> {
        ensure!(
            self.read(&self.seal_path)? == Some(expected.exact()),
            CORRUPT
        );
        Ok(())
    }

    pub(super) fn verify_retirement(&self, expected: &CodexRetirementArtifact) -> Result<()> {
        ensure!(
            self.read(&self.retirement_path)? == Some(expected.exact()),
            CORRUPT
        );
        Ok(())
    }

    pub(super) fn read_seal(&self) -> Result<Option<CodexRetirementSealArtifact>> {
        self.read(&self.seal_path).map(|artifact| {
            artifact
                .map(|(record, identity)| CodexRetirementSealArtifact::from_exact(record, identity))
        })
    }

    pub(super) fn read_retirement(&self) -> Result<Option<CodexRetirementArtifact>> {
        self.read(&self.retirement_path).map(|artifact| {
            artifact.map(|(record, identity)| CodexRetirementArtifact::from_exact(record, identity))
        })
    }

    pub(super) fn verify_seal_only(&self, expected: &CodexRetirementSealArtifact) -> Result<()> {
        self.verify_seal(expected)?;
        ensure!(self.read_retirement()?.is_none(), CORRUPT);
        self.verify_seal(expected)
    }

    pub(super) fn snapshot_path(&self) -> PathBuf {
        self.directory
            .path()
            .join(super::constants::SNAPSHOT_FILE_NAME)
    }

    pub(super) fn directory(&self) -> &crate::paths::directory::PrivateDirectory {
        &self.directory
    }

    pub(super) fn writer_lock_path(&self) -> &Path {
        &self.writer_lock_path
    }

    pub(super) fn invocation_authority(&self) -> Result<CodexInvocationAuthority> {
        CodexInvocationAuthority::capture(&self.directory).context(STORAGE)
    }
}

fn scope_matches(directory: &Path, owner: &HookIdentity) -> bool {
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
