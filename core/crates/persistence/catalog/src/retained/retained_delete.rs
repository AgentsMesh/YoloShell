use super::retained_artifact::RetainedArtifact;
use super::secure_vfs_identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result};
use std::ffi::{CStr, CString, OsString};
use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

const DELETE_MARKER: &str = ".delete-";

pub(super) fn remove_artifact(artifact: &RetainedArtifact) -> Result<()> {
    let Some(expected) = artifact.expected() else {
        anyhow::ensure!(
            artifact.entry().is_missing()?,
            "refuse to delete retained SQLite artifact without an expected identity"
        );
        return Ok(());
    };
    run_test_hook(artifact.path())?;
    if remove_expected(artifact.entry(), expected)? {
        artifact.clear(expected)?;
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static DELETE_TEST_HOOK: std::cell::RefCell<Option<DeleteTestHook>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(test)]
type DeleteTestHook = Box<dyn FnOnce(&std::path::Path) -> Result<()>>;

#[cfg(test)]
pub(super) fn set_test_hook<F>(hook: F)
where
    F: FnOnce(&std::path::Path) -> Result<()> + 'static,
{
    DELETE_TEST_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

fn run_test_hook(path: &std::path::Path) -> Result<()> {
    #[cfg(test)]
    return DELETE_TEST_HOOK.with(|slot| match slot.borrow_mut().take() {
        Some(hook) => hook(path),
        None => Ok(()),
    });
    #[cfg(not(test))]
    let _ = path;
    #[cfg(not(test))]
    Ok(())
}

pub(super) fn remove_descriptor(entry: &PrivateFileEntry, expected: FileIdentity) -> Result<bool> {
    remove_expected(entry, expected)
}

fn remove_expected(entry: &PrivateFileEntry, expected: FileIdentity) -> Result<bool> {
    let temporary = temporary_name(entry)?;
    match rename_exclusive(
        entry.parent_descriptor(),
        entry.file_name(),
        temporary.as_c_str(),
    ) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error).context("isolate retained SQLite artifact for deletion"),
    }
    let actual = inspect_isolated(entry, temporary.as_c_str());
    if actual
        .as_ref()
        .is_err_and(|_| restore(entry, temporary.as_c_str()).is_err())
    {
        return actual.map(|_| false);
    }
    let actual = actual?;
    if actual != expected {
        restore(entry, temporary.as_c_str())?;
        anyhow::bail!("retained SQLite artifact changed before deletion");
    }
    unlink_isolated(entry, temporary.as_c_str())?;
    sync_parent(entry)?;
    Ok(true)
}

fn inspect_isolated(entry: &PrivateFileEntry, name: &CStr) -> Result<FileIdentity> {
    let flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK;
    let descriptor = unsafe { libc::openat(entry.parent_descriptor(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("open isolated SQLite artifact");
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    FileIdentity::inspect(&file.metadata()?, entry.path())
}

fn unlink_isolated(entry: &PrivateFileEntry, name: &CStr) -> Result<()> {
    let result = unsafe { libc::unlinkat(entry.parent_descriptor(), name.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context("unlink isolated SQLite artifact")
    }
}

fn restore(entry: &PrivateFileEntry, temporary: &CStr) -> Result<()> {
    rename_exclusive(entry.parent_descriptor(), temporary, entry.file_name())
        .context("restore rejected SQLite replacement")?;
    sync_parent(entry)
}

fn sync_parent(entry: &PrivateFileEntry) -> Result<()> {
    run_sync_test_hook()?;
    let result = unsafe { libc::fsync(entry.parent_descriptor()) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context("sync retained SQLite parent")
    }
}

#[cfg(test)]
thread_local! {
    static SYNC_TEST_HOOK: std::cell::RefCell<Option<SyncTestHook>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(test)]
type SyncTestHook = Box<dyn FnOnce() -> Result<()>>;

#[cfg(test)]
pub(super) fn set_sync_test_hook<F>(hook: F)
where
    F: FnOnce() -> Result<()> + 'static,
{
    SYNC_TEST_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

fn run_sync_test_hook() -> Result<()> {
    #[cfg(test)]
    return SYNC_TEST_HOOK.with(|slot| match slot.borrow_mut().take() {
        Some(hook) => hook(),
        None => Ok(()),
    });
    #[cfg(not(test))]
    Ok(())
}

fn temporary_name(entry: &PrivateFileEntry) -> Result<CString> {
    let mut name = OsString::from_vec(entry.file_name().to_bytes().to_vec());
    name.push(DELETE_MARKER);
    name.push(uuid::Uuid::new_v4().to_string());
    CString::new(name.as_bytes()).context("SQLite delete name contains a null byte")
}

fn rename_exclusive(parent: libc::c_int, source: &CStr, target: &CStr) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            parent,
            source.as_ptr(),
            parent,
            target.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            parent,
            source.as_ptr(),
            parent,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
