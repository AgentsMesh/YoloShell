use crate::constants::*;
use anyhow::{Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

fn name(value: &std::ffi::OsStr) -> std::io::Result<CString> {
    CString::new(value.as_bytes()).map_err(std::io::Error::other)
}

fn opened(descriptor: RawFd) -> std::io::Result<File> {
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

pub(crate) fn open_directory(parent: RawFd, path: &Path) -> std::io::Result<File> {
    let path = name(path.as_os_str())?;
    opened(unsafe {
        libc::openat(
            parent,
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    })
}

pub(crate) fn open_file(parent: &File, path: &str) -> std::io::Result<File> {
    let path = name(path.as_ref())?;
    opened(unsafe {
        libc::openat(
            parent.as_raw_fd(),
            path.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    })
}

pub(crate) fn create_file(parent: &File, path: &str, mode: u32) -> Result<File> {
    let path = name(path.as_ref())?;
    let file = opened(unsafe {
        libc::openat(
            parent.as_raw_fd(),
            path.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            OWNER_FILE_MODE as libc::c_uint,
        )
    })?;
    ensure!(
        unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) } == 0,
        UNSAFE_CONFIG
    );
    Ok(file)
}

pub(crate) fn create_directory(parent: &File, path: &Path) -> Result<()> {
    let path = name(path.as_os_str())?;
    let result = unsafe {
        libc::mkdirat(
            parent.as_raw_fd(),
            path.as_ptr(),
            OWNER_DIRECTORY_MODE as libc::mode_t,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    parent.sync_all()?;
    Ok(())
}

pub(crate) fn rename(parent: &File, source: &str, target: &str, exchange: bool) -> Result<()> {
    let source = name(source.as_ref())?;
    let target = name(target.as_ref())?;
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            parent.as_raw_fd(),
            source.as_ptr(),
            parent.as_raw_fd(),
            target.as_ptr(),
            if exchange {
                libc::RENAME_SWAP
            } else {
                libc::RENAME_EXCL
            },
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            parent.as_raw_fd(),
            source.as_ptr(),
            parent.as_raw_fd(),
            target.as_ptr(),
            if exchange {
                libc::RENAME_EXCHANGE
            } else {
                libc::RENAME_NOREPLACE
            },
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    parent.sync_all()?;
    Ok(())
}

pub(crate) fn remove(parent: &File, path: &str) -> Result<()> {
    let path = name(path.as_ref())?;
    if unsafe { libc::unlinkat(parent.as_raw_fd(), path.as_ptr(), 0) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    parent.sync_all()?;
    Ok(())
}
