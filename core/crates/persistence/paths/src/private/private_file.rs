use super::constants::PRIVATE_FILE_MODE;
use super::directory::{PrivateDirectory, entry_c_string};
use super::identity::FileIdentity;
#[path = "file/private_file_capacity.rs"]
mod private_file_capacity;
#[path = "file/cleanup/private_file_cleanup.rs"]
mod private_file_cleanup;
#[path = "file/cleanup/private_file_cleanup_io.rs"]
mod private_file_cleanup_io;
#[path = "file/private_file_clone.rs"]
mod private_file_clone;
#[path = "file/private_file_digest.rs"]
mod private_file_digest;
#[path = "file/private_file_error.rs"]
mod private_file_error;
pub use private_file_error::io_kind;
pub(crate) use private_file_error::is_creation_contention;
pub use private_file_io::rename_exclusive_between;
#[path = "file/private_file_exchange.rs"]
mod private_file_exchange;
#[path = "file/exclusive/private_file_exclusive.rs"]
mod private_file_exclusive;
#[path = "file/exclusive/private_file_exclusive_cleanup.rs"]
mod private_file_exclusive_cleanup;
#[cfg(test)]
pub(crate) use private_file_exclusive::install_exclusive_publish_hook;
#[path = "file/private_file_io.rs"]
mod private_file_io;
#[path = "file/private_file_missing.rs"]
pub(crate) mod private_file_missing;
#[path = "file/private_file_names.rs"]
pub(crate) mod private_file_names;
#[doc(hidden)]
pub use private_file_names::{is_atomic_storage_artifact_for, is_storage_artifact};
#[path = "file/private_file_open.rs"]
mod private_file_open;
#[path = "file/publish/private_file_publish.rs"]
mod private_file_publish;
#[path = "file/publish/private_file_publish_verify.rs"]
mod private_file_publish_verify;
#[path = "file/private_file_reclaim.rs"]
mod private_file_reclaim;
#[path = "file/private_file_recoverable_create.rs"]
pub(crate) mod private_file_recoverable_create;
#[path = "file/private_file_relative.rs"]
mod private_file_relative;
#[path = "file/private_file_remove.rs"]
mod private_file_remove;
#[path = "file/retained/private_file_retained.rs"]
mod private_file_retained;
pub use private_file_retained::RetainedPrivateFile;
#[path = "file/retained/private_file_retained_cleanup.rs"]
mod private_file_retained_cleanup;
#[path = "file/retained/private_file_retained_exchange.rs"]
mod private_file_retained_exchange;
#[path = "file/retained/private_file_retained_hooks.rs"]
mod private_file_retained_hooks;
#[path = "file/retained/private_file_retained_recovery.rs"]
mod private_file_retained_recovery;
#[path = "file/retained/private_file_retained_stage.rs"]
mod private_file_retained_stage;
#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) use private_file_clone::set_retained_clone_hook;
#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) use private_file_retained_cleanup::{
    set_retained_cleanup_recycle_hook, set_retained_cleanup_sync_hook,
};
#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) use private_file_retained_hooks::{
    set_retained_exchange_hook, set_retained_install_hook, set_retained_rename_hook,
    set_retained_restore_hook,
};
pub use private_file_retained_stage::RetainedWrite;
#[path = "file/private_file_slot.rs"]
mod private_file_slot;
#[cfg(all(test, feature = "yolo-unit-tests"))]
pub(crate) use private_file_slot::SlotHookGuard;
#[path = "file/private_file_target_lock.rs"]
mod private_file_target_lock;
#[path = "file/temp/private_file_temp.rs"]
mod private_file_temp;
#[cfg(test)]
#[path = "file/temp/private_file_temp_hooks.rs"]
mod private_file_temp_hooks;
#[path = "file/temp/private_file_temp_prepare.rs"]
mod private_file_temp_prepare;
pub(crate) use private_file_target_lock::{ParentLockAuthority, TargetLockAuthority};
#[path = "file/private_file_unmanaged.rs"]
mod private_file_unmanaged;
#[path = "file/private_file_verify.rs"]
mod private_file_verify;
use anyhow::{Context, Result};
use private_file_digest::PrivateFileDigest;
use private_file_io::*;
use private_file_missing::*;
use std::ffi::CString;
use std::fs::File;
use std::path::{Path, PathBuf};
#[derive(Debug)]
pub struct PrivateFileEntry {
    directory: PrivateDirectory,
    name: CString,
    path: PathBuf,
}
#[derive(Debug)]
pub struct OpenedPrivateFile {
    pub(super) file: File,
    pub(super) identity: FileIdentity,
}
#[derive(Debug)]
pub struct TemporaryPrivateFile {
    _process_lock: crate::paths::process_file_lock::ProcessFileLock,
    pub(super) file: File,
    pub(super) identity: FileIdentity,
    digest: PrivateFileDigest,
    previous_digest: Option<PrivateFileDigest>,
    pub(super) name: CString,
    _previous_process_lock: Option<crate::paths::process_file_lock::ProcessFileLock>,
    pub(super) previous: Option<OpenedPrivateFile>,
    pub(super) slot: CString,
}
#[derive(Debug)]
pub struct PrivateDirectoryLock {
    _process_lock: crate::paths::process_file_lock::ProcessFileLock,
    _file: File,
    _turn: Option<crate::paths::process_file_lock::ProcessLockTurn>,
}

include!("file/private_file_capability.rs");

impl PrivateFileEntry {
    pub fn ensure(path: &Path) -> Result<Self> {
        Self::new(path, true)
    }

    pub fn open(path: &Path) -> Result<Self> {
        Self::new(path, false)
    }

    fn new(path: &Path, ensure: bool) -> Result<Self> {
        let parent = path.parent().context("private file path has no parent")?;
        let directory = if ensure {
            PrivateDirectory::ensure(parent)?
        } else {
            PrivateDirectory::open(parent)?
        };
        let file_name = path.file_name().context("private file path has no name")?;
        Ok(Self {
            directory,
            name: entry_c_string(file_name)?,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn remove_private_file(path: &Path) -> Result<()> {
    match PrivateFileEntry::open(path) {
        Ok(entry) => entry.remove(),
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/private/private_file_test_suite.rs"]
mod test_suite;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/private/private_file_unlink_recovery_tests.rs"]
mod unlink_recovery_tests;
