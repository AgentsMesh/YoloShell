use super::{FileIdentity, FileTimestamp, ObjectIdentity, Result};
use crate::paths::constants::{EXPECTED_FILE_LINK_COUNT, PERMISSION_MODE_MASK, PRIVATE_FILE_MODE};
use std::path::Path;

#[cfg(all(unix, not(target_os = "macos")))]
const UNAVAILABLE_TIMESTAMP_COMPONENT: i64 = 0;

impl FileIdentity {
    pub(crate) fn inspect_stat(stat: &libc::stat, path: &Path, max_bytes: u64) -> Result<Self> {
        let mode = u32::from(stat.st_mode);
        let file_type_mask = u32::from(libc::S_IFMT);
        let regular_file = u32::from(libc::S_IFREG);
        anyhow::ensure!(
            mode & file_type_mask == regular_file,
            "state is not a regular file: {}",
            path.display()
        );
        anyhow::ensure!(
            stat.st_uid == unsafe { libc::geteuid() },
            "state owner mismatch: {}",
            path.display()
        );
        anyhow::ensure!(
            u64::from(stat.st_nlink) == EXPECTED_FILE_LINK_COUNT,
            "state has unexpected hard links: {}",
            path.display()
        );
        anyhow::ensure!(
            mode & PERMISSION_MODE_MASK == PRIVATE_FILE_MODE,
            "state permissions are unsafe: {}",
            path.display()
        );
        let length = u64::try_from(stat.st_size)?;
        anyhow::ensure!(
            length <= max_bytes,
            "state exceeds byte limit: {}",
            path.display()
        );
        Ok(Self {
            device: u64::try_from(stat.st_dev)?,
            inode: stat.st_ino,
            length,
            modified: modified_time(stat),
            changed: changed_time(stat),
            birth: birth_time(stat),
        })
    }
}

impl ObjectIdentity {
    pub fn inspect_stat(stat: &libc::stat) -> Result<Self> {
        Ok(Self {
            device: u64::try_from(stat.st_dev)?,
            inode: stat.st_ino,
            birth: birth_time(stat),
        })
    }
}

fn modified_time(stat: &libc::stat) -> FileTimestamp {
    FileTimestamp::new(stat.st_mtime, stat.st_mtime_nsec)
}

fn changed_time(stat: &libc::stat) -> FileTimestamp {
    FileTimestamp::new(stat.st_ctime, stat.st_ctime_nsec)
}

#[cfg(target_os = "macos")]
fn birth_time(stat: &libc::stat) -> FileTimestamp {
    FileTimestamp::new(stat.st_birthtime, stat.st_birthtime_nsec)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn birth_time(_stat: &libc::stat) -> FileTimestamp {
    FileTimestamp::new(
        UNAVAILABLE_TIMESTAMP_COMPONENT,
        UNAVAILABLE_TIMESTAMP_COMPONENT,
    )
}
