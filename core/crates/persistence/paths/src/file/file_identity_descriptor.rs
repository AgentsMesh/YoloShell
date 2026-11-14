use super::{FileIdentity, FileTimestamp, ObjectIdentity};
use crate::paths::constants::*;
#[cfg(target_os = "linux")]
use anyhow::Context;
use anyhow::{Result, ensure};
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(super) struct DescriptorIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    links: u64,
    length: u64,
    modified: FileTimestamp,
    changed: FileTimestamp,
    birth: FileTimestamp,
}

impl DescriptorIdentity {
    pub(super) fn file_identity(&self, path: &Path, max_bytes: u64) -> Result<FileIdentity> {
        ensure!(
            self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG),
            "state is not a regular file: {}",
            path.display()
        );
        ensure!(
            self.uid == unsafe { libc::geteuid() },
            "state owner mismatch: {}",
            path.display()
        );
        ensure!(
            self.links == EXPECTED_FILE_LINK_COUNT,
            "state has unexpected hard links: {}",
            path.display()
        );
        ensure!(
            self.mode & PERMISSION_MODE_MASK == PRIVATE_FILE_MODE,
            "state permissions are unsafe: {}",
            path.display()
        );
        ensure!(
            self.length <= max_bytes,
            "state exceeds byte limit: {}",
            path.display()
        );
        Ok(FileIdentity {
            device: self.device,
            inode: self.inode,
            length: self.length,
            modified: self.modified,
            changed: self.changed,
            birth: self.birth,
        })
    }

    pub(super) fn object_identity(&self) -> ObjectIdentity {
        ObjectIdentity {
            device: self.device,
            inode: self.inode,
            birth: self.birth,
        }
    }
}

#[cfg(target_os = "macos")]
pub(super) fn inspect(file: &File) -> Result<DescriptorIdentity> {
    use std::os::macos::fs::MetadataExt;
    let value = file.metadata()?;
    ensure!(
        valid_birth(value.st_birthtime(), value.st_birthtime_nsec()),
        "descriptor birth identity is unavailable"
    );
    Ok(DescriptorIdentity {
        device: value.dev(),
        inode: value.ino(),
        mode: value.mode(),
        uid: value.uid(),
        links: value.nlink(),
        length: value.len(),
        modified: FileTimestamp::new(value.mtime(), value.mtime_nsec()),
        changed: FileTimestamp::new(value.ctime(), value.ctime_nsec()),
        birth: FileTimestamp::new(value.st_birthtime(), value.st_birthtime_nsec()),
    })
}

#[cfg(target_os = "linux")]
pub(super) fn inspect(file: &File) -> Result<DescriptorIdentity> {
    use std::mem::MaybeUninit;
    use std::os::fd::AsRawFd;
    let mut value = MaybeUninit::<libc::statx>::uninit();
    let result = unsafe {
        libc::statx(
            file.as_raw_fd(),
            c"".as_ptr(),
            libc::AT_EMPTY_PATH | libc::AT_SYMLINK_NOFOLLOW,
            required_linux_mask(),
            value.as_mut_ptr(),
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("inspect descriptor with statx");
    }
    from_linux_statx(unsafe { value.assume_init() }, file)
}

#[cfg(target_os = "linux")]
fn from_linux_statx(value: libc::statx, file: &File) -> Result<DescriptorIdentity> {
    ensure!(
        value.stx_mask & required_linux_mask() == required_linux_mask(),
        "descriptor statx identity is incomplete"
    );
    ensure!(
        valid_birth(value.stx_btime.tv_sec, i64::from(value.stx_btime.tv_nsec)),
        "descriptor birth identity is unavailable"
    );
    let device = linux_device(value.stx_dev_major, value.stx_dev_minor);
    let metadata_device = file.metadata()?.dev();
    ensure!(
        device == metadata_device,
        "descriptor statx device mismatch"
    );
    Ok(DescriptorIdentity {
        device,
        inode: value.stx_ino,
        mode: u32::from(value.stx_mode),
        uid: value.stx_uid,
        links: u64::from(value.stx_nlink),
        length: value.stx_size,
        modified: linux_timestamp(value.stx_mtime)?,
        changed: linux_timestamp(value.stx_ctime)?,
        birth: linux_timestamp(value.stx_btime)?,
    })
}

#[cfg(target_os = "linux")]
const DEVICE_MINOR_LOW_MASK: u64 = 0xff;
#[cfg(target_os = "linux")]
const DEVICE_MAJOR_LOW_MASK: u64 = 0xfff;
#[cfg(target_os = "linux")]
const DEVICE_MINOR_HIGH_SHIFT: u32 = 12;
#[cfg(target_os = "linux")]
const DEVICE_MAJOR_HIGH_SHIFT: u32 = 32;
#[cfg(target_os = "linux")]
const DEVICE_MAJOR_LOW_SHIFT: u32 = 8;

#[cfg(target_os = "linux")]
const fn linux_device(major: u32, minor: u32) -> u64 {
    let major = major as u64;
    let minor = minor as u64;
    (minor & DEVICE_MINOR_LOW_MASK)
        | ((major & DEVICE_MAJOR_LOW_MASK) << DEVICE_MAJOR_LOW_SHIFT)
        | ((minor & !DEVICE_MINOR_LOW_MASK) << DEVICE_MINOR_HIGH_SHIFT)
        | ((major & !DEVICE_MAJOR_LOW_MASK) << DEVICE_MAJOR_HIGH_SHIFT)
}

#[cfg(target_os = "linux")]
const fn required_linux_mask() -> u32 {
    libc::STATX_BASIC_STATS | libc::STATX_BTIME
}

#[cfg(target_os = "linux")]
fn linux_timestamp(value: libc::statx_timestamp) -> Result<FileTimestamp> {
    let nanoseconds = i64::from(value.tv_nsec);
    ensure!(
        valid_nanoseconds(nanoseconds),
        "descriptor timestamp is invalid"
    );
    Ok(FileTimestamp::new(value.tv_sec, nanoseconds))
}

fn valid_birth(seconds: i64, nanoseconds: i64) -> bool {
    seconds > 0 && valid_nanoseconds(nanoseconds)
}

fn valid_nanoseconds(nanoseconds: i64) -> bool {
    u64::try_from(nanoseconds)
        .is_ok_and(|value| value < crate::runtime_constants::NANOSECONDS_PER_SECOND)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn inspect(_file: &File) -> Result<DescriptorIdentity> {
    anyhow::bail!("descriptor birth identity is unsupported")
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/file_identity/file_identity_descriptor_tests.rs"]
mod tests;
