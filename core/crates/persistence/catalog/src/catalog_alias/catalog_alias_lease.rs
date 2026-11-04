use crate::paths::directory::PrivateDirectory;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use crate::paths::process_file_lock::{Mode, ProcessFileLock};
use anyhow::{Context, Result};
use std::ffi::CString;
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) const SHARD_LEASE_FILE: &str = ".lease";
const LEASE_MAX_BYTES: u64 = 0;
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(8);
const RETRY_INTERVAL: Duration = Duration::from_millis(5);
const ACQUIRE_ERROR: &str = "catalog alias shard lease remained busy";
const LOCK_CONTEXT: &str = "lock catalog alias shard lease";

pub(super) struct CatalogAliasShard {
    directory: Arc<PrivateDirectory>,
    _lease: CatalogAliasLease,
}

pub(super) struct CatalogAliasLease {
    entry: PrivateFileEntry,
    _process_lock: ProcessFileLock,
    _file: File,
    identity: FileIdentity,
}

impl CatalogAliasShard {
    pub(super) fn retain(main: &PrivateFileEntry) -> Result<Self> {
        let root = super::catalog_alias_location::root(main)?;
        Self::retain_in_root(main.path(), root)
    }

    fn retain_in_root(catalog: &Path, root: PrivateDirectory) -> Result<Self> {
        let shard_name = crate::coordination::catalog_alias_shard_name(catalog)?;
        let deadline = Instant::now() + ACQUIRE_TIMEOUT;
        loop {
            if let Some(lifecycle) = crate::coordination::try_lifecycle(&root)? {
                run_retain_test_hook(&root);
                let aliases = crate::coordination::catalog_alias_root(&root)?;
                let directory = aliases.ensure_descendant(&aliases.path().join(&shard_name))?;
                if let Some(lease) = CatalogAliasLease::try_shared(&directory)? {
                    drop(lifecycle);
                    return Ok(Self {
                        directory: Arc::new(directory),
                        _lease: lease,
                    });
                }
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                anyhow::bail!(ACQUIRE_ERROR);
            };
            std::thread::sleep(remaining.min(RETRY_INTERVAL));
        }
    }

    pub(super) fn directory(&self) -> &PrivateDirectory {
        &self.directory
    }

    #[cfg(test)]
    pub(super) fn retain_for_test(catalog: &Path, root: PrivateDirectory) -> Result<Self> {
        Self::retain_in_root(catalog, root)
    }
}

fn run_retain_test_hook(root: &PrivateDirectory) {
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    super::catalog_alias_maintenance_hooks::run_retain_hook(root.path());
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    let _ = root;
}

impl CatalogAliasLease {
    pub(super) fn try_shared(directory: &PrivateDirectory) -> Result<Option<Self>> {
        Self::try_lock(directory, Mode::Shared)
    }

    pub(super) fn try_exclusive(directory: &PrivateDirectory) -> Result<Option<Self>> {
        Self::try_lock(directory, Mode::Exclusive)
    }

    fn try_lock(directory: &PrivateDirectory, mode: Mode) -> Result<Option<Self>> {
        let path = directory.path().join(SHARD_LEASE_FILE);
        let entry = PrivateFileEntry::from_directory(directory.try_clone()?, &path)?;
        let opened = match entry.open_rw_or_create(LEASE_MAX_BYTES) {
            Ok(opened) => opened,
            Err(error)
                if crate::paths::private_file::io_kind(&error)
                    == Some(std::io::ErrorKind::WouldBlock) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let identity = opened.identity();
        let Some(process_lock) =
            ProcessFileLock::try_lock(opened.file(), identity.object_identity(), mode)
                .context(LOCK_CONTEXT)?
        else {
            return Ok(None);
        };
        entry.verify_current_unchanged(identity, LEASE_MAX_BYTES)?;
        directory.verify_path()?;
        let (file, identity) = opened.into_parts();
        Ok(Some(Self {
            entry,
            _file: file,
            _process_lock: process_lock,
            identity,
        }))
    }

    pub(super) fn verify(&self, directory: &PrivateDirectory) -> Result<()> {
        anyhow::ensure!(
            self.entry.path().parent() == Some(directory.path()),
            "catalog alias shard lease parent changed"
        );
        self.entry
            .verify_current_unchanged(self.identity, LEASE_MAX_BYTES)
    }

    pub(super) fn unlink_owned(&self, directory: &PrivateDirectory) -> Result<()> {
        self.verify(directory)?;
        let name = CString::new(
            self.entry
                .path()
                .file_name()
                .context("catalog alias shard lease has no name")?
                .as_bytes(),
        )?;
        let result = unsafe { libc::unlinkat(directory.descriptor(), name.as_ptr(), 0) };
        if result != 0 {
            return Err(std::io::Error::last_os_error())
                .context("unlink catalog alias shard lease");
        }
        directory.sync()
    }
}
