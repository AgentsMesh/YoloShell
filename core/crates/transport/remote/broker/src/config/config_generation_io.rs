use super::super::constants::*;
use anyhow::{Context, Result, ensure};
use std::ffi::CStr;
use std::fs::{File, Metadata};
use std::io::Write;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GenerationIdentity {
    device: u64,
    inode: u64,
}

impl GenerationIdentity {
    pub(super) fn inspect(metadata: &Metadata, expected_mode: u32) -> Result<Self> {
        ensure!(
            metadata.is_dir(),
            "broker generation entry is not a directory"
        );
        ensure!(
            metadata.uid() == unsafe { libc::geteuid() },
            "broker generation directory owner mismatch"
        );
        ensure!(
            metadata.mode() & PERMISSION_BITS_MASK == expected_mode,
            "broker generation directory mode mismatch"
        );
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub(super) fn matches(self, metadata: &Metadata) -> bool {
        self.device == metadata.dev() && self.inode == metadata.ino()
    }

    pub(super) fn matches_stat(self, metadata: &libc::stat) -> bool {
        #[cfg(target_os = "macos")]
        {
            u64::try_from(metadata.st_dev).ok() == Some(self.device)
                && metadata.st_ino == self.inode
        }
        #[cfg(target_os = "linux")]
        {
            metadata.st_dev == self.device && metadata.st_ino == self.inode
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            false
        }
    }
}

pub(super) fn open_directory_at(parent: RawFd, name: &CStr) -> std::io::Result<File> {
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::openat(parent, name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

pub(super) fn inspect_at(parent: RawFd, name: &CStr) -> std::io::Result<Option<libc::stat>> {
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
        Ok(Some(unsafe { metadata.assume_init() }))
    } else {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        }
    }
}

pub(super) fn create_directory_at(parent: RawFd, name: &CStr) -> std::io::Result<bool> {
    let mode = libc::mode_t::try_from(PRIVATE_DIRECTORY_MODE).map_err(std::io::Error::other)?;
    let result = unsafe { libc::mkdirat(parent, name.as_ptr(), mode) };
    if result == 0 {
        Ok(true)
    } else {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            Ok(false)
        } else {
            Err(error)
        }
    }
}

pub(super) fn set_directory_mode(directory: &File, path: &Path, mode: u32) -> Result<()> {
    let mode = libc::mode_t::try_from(mode)?;
    let result = unsafe { libc::fchmod(directory.as_raw_fd(), mode) };
    if result != 0 {
        Err(std::io::Error::last_os_error()).context("set broker generation directory mode")
    } else {
        crate::private_acl::clear(directory, path)
    }
}

pub(super) fn try_lock(
    directory: &File,
) -> Result<Option<crate::paths::process_file_lock::ProcessFileLock>> {
    let identity = crate::paths::identity::ObjectIdentity::from_file(directory)?
        .context("broker generation staging birth identity is unavailable")?;
    crate::paths::process_file_lock::ProcessFileLock::try_lock(
        directory,
        identity,
        crate::paths::process_file_lock::Mode::Exclusive,
    )
    .context("lock broker generation staging slot")
}

pub(super) fn write_file_at(
    directory: RawFd,
    name: &CStr,
    path: &Path,
    bytes: &[u8],
) -> Result<()> {
    let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let mode = libc::c_uint::try_from(IMMUTABLE_FILE_MODE)?;
    let descriptor = unsafe { libc::openat(directory, name.as_ptr(), flags, mode) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("create broker generation file");
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    let chmod = unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) };
    if chmod != 0 {
        return Err(std::io::Error::last_os_error()).context("seal broker generation file");
    }
    crate::private_acl::clear(&file, path)?;
    file.write_all(bytes)?;
    file.sync_all().context("sync broker generation file")
}

pub(super) fn unlink_file_at(directory: RawFd, name: &CStr) {
    unsafe {
        libc::unlinkat(directory, name.as_ptr(), 0);
    }
}

pub(super) fn rename_exclusive_at(parent: RawFd, source: &CStr, target: &CStr) -> Result<bool> {
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
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err(anyhow::anyhow!(
        "exclusive generation publish is unsupported"
    ));
    if result == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        Ok(false)
    } else {
        Err(error).context("publish broker config generation")
    }
}
