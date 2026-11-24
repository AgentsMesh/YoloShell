use super::constants::*;
use super::{CgroupDirectoryIdentity, LinuxContainmentError};
use std::ffi::{CStr, CString, OsStr};
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

pub(super) fn open_root(path: &Path) -> Result<File, LinuxContainmentError> {
    if !path.is_absolute() {
        return Err(LinuxContainmentError::Identity(
            "delegated root must be absolute",
        ));
    }
    let mut current = open_directory(libc::AT_FDCWD, c_string(OsStr::new("/"))?.as_c_str())?;
    for component in path.components().skip(1) {
        let Component::Normal(name) = component else {
            return Err(LinuxContainmentError::Identity(
                "delegated root is not normalized",
            ));
        };
        current = open_directory(current.as_raw_fd(), c_string(name)?.as_c_str())?;
    }
    require_cgroup2(&current)?;
    Ok(current)
}

pub(super) fn open_leaf(root: &File, name: &str) -> Result<File, LinuxContainmentError> {
    validate_leaf_name(name)?;
    open_directory(root.as_raw_fd(), c_string(OsStr::new(name))?.as_c_str()).map_err(Into::into)
}

pub(super) fn create_leaf(root: &File, name: &str) -> Result<File, LinuxContainmentError> {
    validate_leaf_name(name)?;
    let name = c_string(OsStr::new(name))?;
    let result = unsafe { libc::mkdirat(root.as_raw_fd(), name.as_ptr(), CREATE_DIRECTORY_MODE) };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    open_directory(root.as_raw_fd(), name.as_c_str()).map_err(Into::into)
}

pub(super) fn remove_leaf(root: &File, name: &str) -> Result<(), LinuxContainmentError> {
    validate_leaf_name(name)?;
    let name = c_string(OsStr::new(name))?;
    let result = unsafe { libc::unlinkat(root.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().into())
    }
}

pub(super) fn remove_exact_leaf(
    root: &File,
    retained: &File,
    name: &str,
) -> Result<(), LinuxContainmentError> {
    let current = open_leaf(root, name)?;
    if identity(&current)? != identity(retained)? {
        return Err(LinuxContainmentError::Identity(
            "session cgroup leaf was replaced before removal",
        ));
    }
    remove_leaf(root, name)
}

pub(super) fn identity(file: &File) -> Result<CgroupDirectoryIdentity, LinuxContainmentError> {
    let metadata = file.metadata()?;
    if !metadata.is_dir() {
        return Err(LinuxContainmentError::Identity(
            "cgroup descriptor is not a directory",
        ));
    }
    Ok(CgroupDirectoryIdentity {
        mount_id: mount_id(file.as_raw_fd())?,
        inode: metadata.ino(),
    })
}

pub(super) fn validate_leaf_name(name: &str) -> Result<(), LinuxContainmentError> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    valid
        .then_some(())
        .ok_or(LinuxContainmentError::Identity("invalid cgroup leaf name"))
}

fn mount_id(descriptor: RawFd) -> Result<u64, LinuxContainmentError> {
    let path = format!("/proc/self/fdinfo/{descriptor}");
    let source = File::open(path)?;
    let mut bytes = Vec::new();
    source
        .take((MAX_FDINFO_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_FDINFO_BYTES {
        return Err(LinuxContainmentError::Identity("fdinfo is oversized"));
    }
    let content = std::str::from_utf8(&bytes)
        .map_err(|_| LinuxContainmentError::Identity("fdinfo is not UTF-8"))?;
    content
        .lines()
        .find_map(|line| line.strip_prefix(FDINFO_MOUNT_ID_PREFIX))
        .and_then(|value| value.trim().parse().ok())
        .ok_or(LinuxContainmentError::Identity(
            "fdinfo has no mount identity",
        ))
}

fn require_cgroup2(file: &File) -> Result<(), LinuxContainmentError> {
    let mut state = std::mem::MaybeUninit::<libc::statfs>::uninit();
    let result = unsafe { libc::fstatfs(file.as_raw_fd(), state.as_mut_ptr()) };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let state = unsafe { state.assume_init() };
    if state.f_type != CGROUP2_SUPER_MAGIC {
        return Err(LinuxContainmentError::Unavailable(
            "delegated root is not on cgroup v2",
        ));
    }
    Ok(())
}

fn open_directory(parent: RawFd, name: &CStr) -> std::io::Result<File> {
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::openat(parent, name.as_ptr(), flags, OPEN_MODE_UNUSED) };
    if descriptor == INVALID_DESCRIPTOR {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

fn c_string(value: &OsStr) -> Result<CString, LinuxContainmentError> {
    CString::new(value.as_bytes())
        .map_err(|_| LinuxContainmentError::Identity("cgroup path contains a null byte"))
}
