use super::private_file_cleanup_io::{DirectoryStream, lock_directory};
use super::private_file_io::{open_named, read_flags, rename_exclusive_at, write_flags};
use super::private_file_missing::entry_is_missing;
use super::private_file_names::quarantine_name;
use super::{PrivateDirectoryLock, PrivateFileEntry};
use crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES;
use anyhow::{Context, Result};
use std::ffi::{CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
impl PrivateFileEntry {
    pub fn lock_and_cleanup(&self, prefix: &str, suffix: &str) -> Result<PrivateDirectoryLock> {
        self.lock_and_cleanup_bounded_with(
            prefix,
            suffix,
            MAX_PRIVATE_DIRECTORY_ENTRIES,
            |_| Ok(()),
        )
    }

    pub fn lock_and_cleanup_bounded(
        &self,
        prefix: &str,
        suffix: &str,
        max_entries: usize,
    ) -> Result<PrivateDirectoryLock> {
        self.lock_and_cleanup_bounded_with(prefix, suffix, max_entries, |_| Ok(()))
    }

    pub(crate) fn lock_and_cleanup_bounded_with<F>(
        &self,
        prefix: &str,
        suffix: &str,
        max_entries: usize,
        mut before_quarantine: F,
    ) -> Result<PrivateDirectoryLock>
    where
        F: FnMut(&Path) -> Result<()>,
    {
        self.directory.verify_path()?;
        let (file, process_lock) = lock_directory(self.directory.descriptor())?;
        let lock = PrivateDirectoryLock {
            _file: file,
            _process_lock: process_lock,
            _turn: None,
        };
        self.cleanup_stale(prefix, suffix, max_entries, &mut before_quarantine)?;
        Ok(lock)
    }

    fn cleanup_stale<F>(
        &self,
        prefix: &str,
        suffix: &str,
        max_entries: usize,
        hook: &mut F,
    ) -> Result<()>
    where
        F: FnMut(&Path) -> Result<()>,
    {
        let mut stream = DirectoryStream::open(self.directory.descriptor())?;
        let mut inspected = 0_usize;
        let mut changed = false;
        while let Some(name) = stream.next_name()? {
            inspected = inspected
                .checked_add(1)
                .context("private directory entry count overflow")?;
            anyhow::ensure!(
                inspected <= max_entries,
                "private directory exceeds entry limit"
            );
            if !is_temporary_name(&name, prefix, suffix) {
                continue;
            }
            let source = CString::new(name.as_bytes())?;
            let path = self.path.with_file_name(&name);
            self.quarantine_stale(&source, &path, prefix, hook)?;
            changed = true;
        }
        if changed {
            self.directory.sync()?;
        }
        self.directory.verify_path()
    }

    fn quarantine_stale<F>(
        &self,
        source: &CString,
        path: &Path,
        prefix: &str,
        hook: &mut F,
    ) -> Result<()>
    where
        F: FnMut(&Path) -> Result<()>,
    {
        let first = open_named(
            self.directory.descriptor(),
            source,
            path,
            write_flags(),
            u64::MAX,
        )?;
        let second = open_named(
            self.directory.descriptor(),
            source,
            path,
            read_flags(),
            u64::MAX,
        )?;
        anyhow::ensure!(
            first.identity.unchanged(second.identity),
            "stale temporary file changed"
        );
        let quarantine = quarantine_name(&self.name, prefix, source)?;
        anyhow::ensure!(
            entry_is_missing(&self.directory, &quarantine)?,
            "private quarantine slot is already occupied"
        );
        hook(path)?;
        rename_exclusive_at(self.directory.descriptor(), source, &quarantine)
            .context("quarantine stale private file")?;
        let validation = self.verify_quarantine(&quarantine, first.identity, path);
        if let Err(error) = validation {
            return self.restore_quarantine(source, &quarantine, error);
        }
        first
            .file
            .set_len(0)
            .context("truncate quarantined private file")?;
        first
            .file
            .sync_all()
            .context("sync quarantined private file")?;
        let current = open_named(
            self.directory.descriptor(),
            &quarantine,
            path,
            read_flags(),
            u64::MAX,
        )?;
        anyhow::ensure!(
            current.identity.same_object(first.identity),
            "quarantine file changed"
        );
        Ok(())
    }

    fn verify_quarantine(
        &self,
        quarantine: &CString,
        expected: crate::paths::identity::FileIdentity,
        path: &Path,
    ) -> Result<()> {
        let current = open_named(
            self.directory.descriptor(),
            quarantine,
            path,
            read_flags(),
            u64::MAX,
        )?;
        anyhow::ensure!(
            current.identity.same_content_version(expected),
            "quarantined private file changed"
        );
        Ok(())
    }

    fn restore_quarantine(
        &self,
        source: &CString,
        quarantine: &CString,
        cause: anyhow::Error,
    ) -> Result<()> {
        let restored = rename_exclusive_at(self.directory.descriptor(), quarantine, source);
        self.directory.sync()?;
        match restored {
            Ok(()) => Err(cause.context("stale private file quarantine was rolled back")),
            Err(error) => Err(cause.context(format!(
                "stale private replacement was retained in quarantine: {error}"
            ))),
        }
    }
}

fn is_temporary_name(name: &OsStr, prefix: &str, suffix: &str) -> bool {
    let Some(value) = name
        .to_str()
        .and_then(|name| name.strip_prefix(prefix))
        .and_then(|name| name.strip_suffix(suffix))
    else {
        return false;
    };
    uuid::Uuid::parse_str(value)
        .is_ok_and(|identifier| identifier.hyphenated().to_string() == value)
}
