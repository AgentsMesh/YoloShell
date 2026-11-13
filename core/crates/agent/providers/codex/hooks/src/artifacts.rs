use super::constants::{ARTIFACTS_DIRECTORY_NAME, OUTBOX_FILE_NAME, SNAPSHOT_FILE_NAME, errors};
use super::retirement_constants::{FILE_NAME, SEAL_FILE_NAME, WRITER_LOCK_FILE_NAME};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexHookArtifacts {
    directory: PathBuf,
    invocation_id: Uuid,
}

impl CodexHookArtifacts {
    pub fn new(
        paths: &crate::paths::Paths,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        invocation_id: Uuid,
    ) -> Result<Self> {
        validate_ids(terminal_id, runtime_epoch, invocation_id)?;
        Ok(Self {
            directory: Self::runtime_directory(paths, terminal_id, runtime_epoch)?
                .join(invocation_id.to_string()),
            invocation_id,
        })
    }

    pub fn runtime_directory(
        paths: &crate::paths::Paths,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> Result<PathBuf> {
        ensure!(
            !terminal_id.is_nil() && !runtime_epoch.is_nil(),
            errors::ARTIFACT_PATH_INVALID
        );
        crate::paths::validate_home(paths.home())
            .map_err(|_| anyhow::anyhow!(errors::ARTIFACT_PATH_INVALID))?;
        Ok(paths
            .session_dir(terminal_id)
            .join(ARTIFACTS_DIRECTORY_NAME)
            .join(runtime_epoch.to_string()))
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn invocation_id(&self) -> Uuid {
        self.invocation_id
    }

    pub fn outbox_path(&self) -> PathBuf {
        self.directory.join(OUTBOX_FILE_NAME)
    }

    pub fn intake_directory(&self) -> PathBuf {
        self.directory.join(super::intake_constants::DIRECTORY_NAME)
    }

    pub fn snapshot_path(&self) -> PathBuf {
        self.directory.join(SNAPSHOT_FILE_NAME)
    }

    pub(crate) fn retirement_path(&self) -> PathBuf {
        self.directory.join(FILE_NAME)
    }

    pub(crate) fn retirement_seal_path(&self) -> PathBuf {
        self.directory.join(SEAL_FILE_NAME)
    }

    pub(crate) fn retirement_writer_lock_path(&self) -> PathBuf {
        self.directory.join(WRITER_LOCK_FILE_NAME)
    }
}

fn validate_ids(terminal_id: Uuid, runtime_epoch: Uuid, invocation_id: Uuid) -> Result<()> {
    ensure!(
        !terminal_id.is_nil() && !runtime_epoch.is_nil() && !invocation_id.is_nil(),
        errors::ARTIFACT_PATH_INVALID
    );
    Ok(())
}
