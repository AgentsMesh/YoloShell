use super::constants::{NATIVE_ARTIFACTS_DIRECTORY_NAME, NATIVE_REQUEST_SUFFIX};
use super::{CodexNativeRequest, intake_storage};
use crate::paths::directory::PrivateDirectory;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use uuid::Uuid;

const SPOOL_INVALID: &str = "Codex native callback spool changed or is invalid";
const SPOOL_CAPACITY: &str = "Codex native callback spool is full";
const ENTRY_INCREMENT: usize = 1;

pub struct CodexNativeSpool {
    directory: PrivateDirectory,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexNativeOccurrence {
    request: CodexNativeRequest,
    identity: FileIdentity,
}

impl CodexNativeSpool {
    pub fn for_scope(
        paths: &crate::paths::Paths,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> Result<Self> {
        ensure!(
            !terminal_id.is_nil() && !runtime_epoch.is_nil(),
            SPOOL_INVALID
        );
        let session = PrivateDirectory::open(&paths.session_dir(terminal_id))?;
        let directory = session.ensure_descendant(
            &session
                .path()
                .join(NATIVE_ARTIFACTS_DIRECTORY_NAME)
                .join(runtime_epoch.to_string()),
        )?;
        Ok(Self {
            directory,
            terminal_id,
            runtime_epoch,
        })
    }

    pub fn persist(&self, request: &CodexNativeRequest) -> Result<CodexNativeOccurrence> {
        self.validate_scope(request)?;
        let path = self.path(request.event_id);
        let entry = PrivateFileEntry::from_directory(self.directory.try_clone()?, &path)?;
        let _lock = entry.lock_parent_exclusive_wait(
            super::constants::LOCK_TIMEOUT,
            super::constants::LOCK_RETRY,
        )?;
        if let Some(existing) = self.read(request.event_id)? {
            ensure!(existing.request == *request, SPOOL_INVALID);
            return Ok(existing);
        }
        self.verify_capacity()?;
        let identity = intake_storage::create_with_fence(&self.directory, &path, request, || {
            self.directory.verify_path()
        })?;
        let occurrence = CodexNativeOccurrence {
            request: request.clone(),
            identity,
        };
        self.verify(&occurrence)?;
        Ok(occurrence)
    }

    pub fn read(&self, event_id: Uuid) -> Result<Option<CodexNativeOccurrence>> {
        ensure!(!event_id.is_nil(), SPOOL_INVALID);
        let path = self.path(event_id);
        let entry = PrivateFileEntry::from_directory(self.directory.try_clone()?, &path)?;
        if entry.is_missing()? {
            self.directory.verify_path()?;
            return Ok(None);
        }
        let (bytes, identity) = intake_storage::read_bytes(&self.directory, &path)?;
        let request: CodexNativeRequest = serde_json::from_slice(&bytes).context(SPOOL_INVALID)?;
        self.validate_scope(&request)?;
        ensure!(request.event_id == event_id, SPOOL_INVALID);
        Ok(Some(CodexNativeOccurrence { request, identity }))
    }

    pub fn verify(&self, occurrence: &CodexNativeOccurrence) -> Result<()> {
        ensure!(
            self.read(occurrence.request.event_id)?.as_ref() == Some(occurrence),
            SPOOL_INVALID
        );
        Ok(())
    }

    pub fn remove(&self, occurrence: &CodexNativeOccurrence) -> Result<()> {
        self.verify(occurrence)?;
        intake_storage::remove_exact_with_fence(
            &self.directory,
            &self.path(occurrence.request.event_id),
            occurrence.identity,
            || self.directory.verify_path(),
        )
    }

    fn path(&self, event_id: Uuid) -> PathBuf {
        self.directory
            .path()
            .join(format!("{event_id}{NATIVE_REQUEST_SUFFIX}"))
    }

    fn validate_scope(&self, request: &CodexNativeRequest) -> Result<()> {
        request.validate()?;
        ensure!(
            request.terminal_id == self.terminal_id
                && request.runtime_epoch == self.runtime_epoch,
            SPOOL_INVALID
        );
        self.directory.verify_path()
    }

    fn verify_capacity(&self) -> Result<()> {
        let mut stream =
            crate::paths::directory_stream::DirectoryStream::open(self.directory.descriptor())?;
        let mut count = usize::default();
        while stream.next_name()?.is_some() {
            count = count.checked_add(ENTRY_INCREMENT).context(SPOOL_CAPACITY)?;
            ensure!(
                count < super::intake_constants::MAX_PENDING_OCCURRENCES,
                SPOOL_CAPACITY
            );
        }
        self.directory.verify_path()
    }
}

impl CodexNativeOccurrence {
    pub fn request(&self) -> &CodexNativeRequest {
        &self.request
    }
}
