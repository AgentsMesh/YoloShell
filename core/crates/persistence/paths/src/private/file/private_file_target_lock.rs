use super::*;
use crate::paths::process_file_lock::ProcessFileLock;

const TARGET_LOCK_MISMATCH: &str = "target lock authority does not match the private file";

pub(crate) struct TargetLockAuthority<'a> {
    file: &'a File,
    identity: FileIdentity,
    lock: &'a ProcessFileLock,
    path: PathBuf,
}

pub(crate) struct ParentLockAuthority<'a> {
    identity: crate::paths::identity::ObjectIdentity,
    lock: &'a ProcessFileLock,
    path: PathBuf,
}

impl PrivateFileEntry {
    pub(crate) fn parent_lock_authority<'a>(
        &self,
        lock: &'a PrivateDirectoryLock,
    ) -> Result<ParentLockAuthority<'a>> {
        self.directory.verify_descriptor_path()?;
        let identity = self.directory.object_identity()?;
        anyhow::ensure!(
            lock._process_lock.holds_exclusive(identity),
            TARGET_LOCK_MISMATCH
        );
        Ok(ParentLockAuthority {
            identity,
            lock: &lock._process_lock,
            path: self.directory.path().to_path_buf(),
        })
    }

    pub(crate) fn target_lock_authority<'a>(
        &self,
        opened: &'a OpenedPrivateFile,
        lock: &'a ProcessFileLock,
        max_bytes: u64,
    ) -> Result<TargetLockAuthority<'a>> {
        anyhow::ensure!(
            lock.holds_exclusive(opened.identity.object_identity()),
            TARGET_LOCK_MISMATCH
        );
        opened.identity.verify_file_bounded(
            &opened.file,
            opened.identity.length(),
            &self.path,
            max_bytes,
        )?;
        self.verify_current_unchanged(opened.identity, max_bytes)?;
        Ok(TargetLockAuthority {
            file: &opened.file,
            identity: opened.identity,
            lock,
            path: self.path.clone(),
        })
    }
}

impl ParentLockAuthority<'_> {
    pub(in crate::paths) fn require_entry(&self, entry: &PrivateFileEntry) -> Result<()> {
        anyhow::ensure!(
            entry.directory.path() == self.path && self.lock.holds_exclusive(self.identity),
            TARGET_LOCK_MISMATCH
        );
        entry.directory.verify_descriptor_path()?;
        anyhow::ensure!(
            entry.directory.object_identity()? == self.identity,
            TARGET_LOCK_MISMATCH
        );
        entry.directory.verify_descriptor_path()?;
        Ok(())
    }
}

impl TargetLockAuthority<'_> {
    pub(super) fn open_target(
        &self,
        entry: &PrivateFileEntry,
        max_bytes: u64,
    ) -> Result<OpenedPrivateFile> {
        anyhow::ensure!(self.path == entry.path, TARGET_LOCK_MISMATCH);
        anyhow::ensure!(
            self.lock.holds_exclusive(self.identity.object_identity()),
            TARGET_LOCK_MISMATCH
        );
        self.identity.verify_file_bounded(
            self.file,
            self.identity.length(),
            &self.path,
            max_bytes,
        )?;
        entry.verify_current_unchanged(self.identity, max_bytes)?;
        Ok(OpenedPrivateFile {
            file: self.file.try_clone()?,
            identity: self.identity,
        })
    }

    pub(in crate::paths) fn require_expected(&self, expected: Option<FileIdentity>) -> Result<()> {
        anyhow::ensure!(expected == Some(self.identity), TARGET_LOCK_MISMATCH);
        Ok(())
    }
}
