use super::private_file_cleanup_io::{DirectoryStream, lock_directory};
use super::private_file_io::{open_named, read_flags};
use super::private_file_missing::retained_entry_is_missing;
use super::private_file_names::retired_name;
use super::*;
use crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES;

#[derive(Debug)]
pub struct RetainedPrivateFile {
    pub(super) cleanup: RetainedCleanup,
    pub(super) directory: PrivateDirectory,
    pub(super) _process_lock: crate::paths::process_file_lock::ProcessFileLock,
    pub(super) file: File,
    pub(super) identity: FileIdentity,
    pub(super) name: CString,
    pub(super) path: PathBuf,
}

#[derive(Debug)]
pub(super) enum RetainedCleanup {
    Named(Vec<OwnedRetainedName>),
    ParentSyncPending,
    PreserveSyncPending,
    Complete,
}

#[derive(Debug)]
pub(super) struct OwnedRetainedName {
    pub(super) identity: FileIdentity,
    pub(super) name: CString,
}

impl RetainedPrivateFile {
    pub(super) fn verify_named(&self, max_bytes: u64) -> Result<OpenedPrivateFile> {
        let named = open_named(
            self.directory.descriptor(),
            &self.name,
            &self.path,
            read_flags(),
            max_bytes,
        )?;
        anyhow::ensure!(
            named.identity.unchanged(self.identity),
            "retained private staging name changed"
        );
        Ok(named)
    }

    pub(super) fn verify_named_content(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<OpenedPrivateFile> {
        let named = open_named(
            self.directory.descriptor(),
            &self.name,
            &self.path,
            read_flags(),
            max_bytes,
        )?;
        anyhow::ensure!(
            named.identity.same_content_version(expected),
            "retained staging content changed"
        );
        Ok(named)
    }
}

impl PrivateFileEntry {
    pub fn lock_retained_parent(&self) -> Result<PrivateDirectoryLock> {
        let (file, process_lock) = lock_directory(self.directory.descriptor())?;
        let staging = retired_name(&self.name)?;
        let reservation = usize::from(retained_entry_is_missing(&self.directory, &staging)?);
        let mut stream = DirectoryStream::open(self.directory.descriptor())?;
        let mut entries = 0_usize;
        while stream.next_name()?.is_some() {
            entries = entries
                .checked_add(1)
                .context("retained directory entry count overflow")?;
            anyhow::ensure!(
                entries
                    .checked_add(reservation)
                    .context("retained entry reservation overflow")?
                    <= MAX_PRIVATE_DIRECTORY_ENTRIES,
                "retained directory exceeds entry limit"
            );
        }
        Ok(PrivateDirectoryLock {
            _file: file,
            _process_lock: process_lock,
            _turn: None,
        })
    }

    pub fn open_retained(&self, max_bytes: u64) -> Result<OpenedPrivateFile> {
        open_named(
            self.directory.descriptor(),
            &self.name,
            &self.path,
            read_flags(),
            max_bytes,
        )
    }

    pub(crate) fn open_retained_with_lock(
        &self,
        max_bytes: u64,
    ) -> Result<(
        OpenedPrivateFile,
        crate::paths::process_file_lock::ProcessFileLock,
    )> {
        let opened = self.open_retained(max_bytes)?;
        let process_lock = super::private_file_io::lock_private_file(&opened.file)?;
        Ok((opened, process_lock))
    }

    pub fn verify_retained_content(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<OpenedPrivateFile> {
        let current = self.open_retained(max_bytes)?;
        anyhow::ensure!(
            current.identity.same_content_version(expected),
            "retained private file content changed"
        );
        Ok(current)
    }

    pub fn verify_retained_unchanged(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<OpenedPrivateFile> {
        let current = self.open_retained(max_bytes)?;
        anyhow::ensure!(
            current.identity.unchanged(expected),
            "retained private file version changed"
        );
        Ok(current)
    }

    pub(crate) fn verify_retained_path_unchanged(
        &self,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<OpenedPrivateFile> {
        let current = self.verify_retained_unchanged(expected, max_bytes)?;
        self.directory.verify_descriptor_path()?;
        Ok(current)
    }

    pub fn sync_retained_parent(&self) -> Result<()> {
        self.directory.sync()
    }
}

include!("private_file_retained_lock.rs");
