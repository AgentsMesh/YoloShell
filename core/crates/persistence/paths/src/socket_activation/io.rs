use super::constants::*;
use super::{SocketTarget, StageEntry};
use crate::paths::constants::{PERMISSION_MODE_MASK, PRIVATE_FILE_MODE};
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result};
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SocketIdentity {
    device: libc::dev_t,
    inode: libc::ino_t,
}

pub(super) fn inspect_at(
    directory: &PrivateDirectory,
    name: &CString,
    private: bool,
) -> Result<Option<SocketIdentity>> {
    Ok(inspect_state_at(directory, name, private)?.map(|state| state.0))
}

pub(super) fn socket_matches_at(
    directory: &PrivateDirectory,
    name: &CString,
    private: bool,
    identity: SocketIdentity,
    links: u64,
) -> Result<bool> {
    Ok(inspect_state_at(directory, name, private)? == Some((identity, links)))
}

fn inspect_state_at(
    directory: &PrivateDirectory,
    name: &CString,
    private: bool,
) -> Result<Option<(SocketIdentity, u64)>> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory.descriptor(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        return if error.kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error.into())
        };
    }
    let metadata = unsafe { metadata.assume_init() };
    anyhow::ensure!(
        metadata.st_mode & libc::S_IFMT == libc::S_IFSOCK,
        SOCKET_TYPE_ERROR
    );
    anyhow::ensure!(
        metadata.st_uid == unsafe { libc::geteuid() },
        SOCKET_OWNER_ERROR
    );
    if private {
        let mode = u32::from(metadata.st_mode);
        anyhow::ensure!(
            mode & PERMISSION_MODE_MASK == PRIVATE_FILE_MODE,
            SOCKET_MODE_ERROR
        );
    }
    Ok(Some((
        SocketIdentity {
            device: metadata.st_dev,
            inode: metadata.st_ino,
        },
        u64::from(metadata.st_nlink),
    )))
}

pub(super) fn c_name(path: &Path) -> Result<CString> {
    CString::new(
        path.file_name()
            .context("socket path has no file name")?
            .as_bytes(),
    )
    .context("socket file name contains a null byte")
}

pub(super) fn unlink_at(directory: &PrivateDirectory, name: &CString) -> Result<()> {
    let result = unsafe { libc::unlinkat(directory.descriptor(), name.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(UNLINK_ERROR)
    }
}

pub(super) fn chmod_at(directory: &PrivateDirectory, name: &CString) -> Result<()> {
    let mode = libc::mode_t::try_from(PRIVATE_FILE_MODE)?;
    let result = unsafe { libc::fchmodat(directory.descriptor(), name.as_ptr(), mode, 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(CHMOD_ERROR)
    }
}

pub(super) fn link_at(source: &SocketTarget, target: &StageEntry) -> Result<()> {
    anyhow::ensure!(
        source.directory.same_identity(&target.directory),
        STAGE_PARENT_CHANGED_ERROR
    );
    let result = unsafe {
        libc::linkat(
            source.directory.descriptor(),
            source.name.as_ptr(),
            target.directory.descriptor(),
            target.name.as_ptr(),
            0,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(LINK_ERROR)
    }
}

pub(super) fn rename_exclusive(source: &StageEntry, target: &SocketTarget) -> Result<()> {
    anyhow::ensure!(
        source.directory.same_identity(&target.directory),
        STAGE_PARENT_CHANGED_ERROR
    );
    let result = rename_native(
        source.directory.descriptor(),
        &source.name,
        target.directory.descriptor(),
        &target.name,
    );
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(RENAME_ERROR)
    }
}

#[cfg(target_os = "macos")]
fn rename_native(from: RawFd, source: &CString, to: RawFd, target: &CString) -> libc::c_int {
    unsafe {
        libc::renameatx_np(
            from,
            source.as_ptr(),
            to,
            target.as_ptr(),
            libc::RENAME_EXCL,
        )
    }
}

#[cfg(target_os = "linux")]
fn rename_native(from: RawFd, source: &CString, to: RawFd, target: &CString) -> libc::c_int {
    unsafe {
        libc::renameat2(
            from,
            source.as_ptr(),
            to,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    }
}
