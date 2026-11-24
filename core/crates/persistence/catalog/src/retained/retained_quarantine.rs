use super::constants::CATALOG_QUARANTINE_MARKER;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result};
use std::ffi::{CStr, CString, OsString};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(super) fn quarantine_entry(entry: &PrivateFileEntry) -> Result<()> {
    let Some(identity) = inspect(entry)? else {
        return Ok(());
    };
    let target = quarantine_name(entry.path())?;
    rename_exclusive(entry.parent_descriptor(), entry.file_name(), &target)?;
    let quarantined = inspect_name(entry.parent_descriptor(), &target)?
        .context("quarantined catalog artifact disappeared")?;
    anyhow::ensure!(
        identity == quarantined,
        "catalog quarantine identity changed"
    );
    entry.sync_parent()
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct EntryIdentity {
    device: libc::dev_t,
    inode: libc::ino_t,
    mode: libc::mode_t,
    links: libc::nlink_t,
    owner: libc::uid_t,
}

fn inspect(entry: &PrivateFileEntry) -> Result<Option<EntryIdentity>> {
    inspect_name(entry.parent_descriptor(), entry.file_name())
}

fn inspect_name(parent: libc::c_int, name: &CStr) -> Result<Option<EntryIdentity>> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        let metadata = unsafe { metadata.assume_init() };
        return Ok(Some(EntryIdentity {
            device: metadata.st_dev,
            inode: metadata.st_ino,
            mode: metadata.st_mode & libc::S_IFMT,
            links: metadata.st_nlink,
            owner: metadata.st_uid,
        }));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(error.into())
    }
}

fn quarantine_name(path: &Path) -> Result<CString> {
    let mut name = OsString::from(path.file_name().context("catalog artifact has no name")?);
    name.push(CATALOG_QUARANTINE_MARKER);
    name.push(uuid::Uuid::new_v4().to_string());
    CString::new(name.as_bytes()).context("catalog quarantine name contains a null byte")
}

fn rename_exclusive(parent: libc::c_int, source: &CStr, target: &CStr) -> Result<()> {
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
        Err(std::io::Error::last_os_error()).context("quarantine retained catalog artifact")
    }
}
