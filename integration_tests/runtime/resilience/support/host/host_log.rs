use super::*;
use anyhow::Context;
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt, PermissionsExt};

#[path = "host_log_acl.rs"]
mod acl;

const HOST_STDERR_OPEN_CONTEXT: &str = "open private Host stderr log";
const HOST_STDERR_SECURITY_ERROR: &str = "Host stderr log is not a private regular file";
const PERMISSION_MODE_MASK: u32 = 0o7777;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct HostLogIdentity {
    device: u64,
    inode: u64,
}

pub(super) struct HostLog {
    file: File,
    identity: HostLogIdentity,
}

impl HostLog {
    pub(super) fn open(home: &Path) -> anyhow::Result<Self> {
        let path = home.join(HOST_STDERR_FILE_NAME);
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .mode(HOST_STDERR_MODE)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(&path)
            .context(HOST_STDERR_OPEN_CONTEXT)?;
        file.set_permissions(std::fs::Permissions::from_mode(HOST_STDERR_MODE))?;
        acl::clear_and_verify(file.as_raw_fd())?;
        let metadata = file.metadata()?;
        anyhow::ensure!(
            metadata.is_file()
                && metadata.uid() == unsafe { libc::geteuid() }
                && metadata.nlink() == 1
                && metadata.permissions().mode() & PERMISSION_MODE_MASK == HOST_STDERR_MODE,
            HOST_STDERR_SECURITY_ERROR
        );
        Ok(Self {
            identity: HostLogIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            },
            file,
        })
    }

    pub(super) fn child_stderr(&self) -> anyhow::Result<Stdio> {
        Ok(Stdio::from(self.file.try_clone()?))
    }

    pub(super) fn identity(&self) -> HostLogIdentity {
        self.identity
    }

    pub(super) fn tail(&self) -> String {
        self.read_tail()
            .unwrap_or_else(|error| format!("<unavailable: {error}>"))
    }

    fn read_tail(&self) -> std::io::Result<String> {
        let length = self.file.metadata()?.len();
        let limit = u64::try_from(HOST_STDERR_MAX_TAIL_BYTES).unwrap_or(u64::MAX);
        let offset = length.saturating_sub(limit);
        let count = usize::try_from(length - offset).unwrap_or(HOST_STDERR_MAX_TAIL_BYTES);
        let mut bytes = vec![0_u8; count.min(HOST_STDERR_MAX_TAIL_BYTES)];
        let read = self.file.read_at(&mut bytes, offset)?;
        bytes.truncate(read);
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    #[cfg(test)]
    pub(super) fn append_for_test(&self, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        self.file.try_clone()?.write_all(bytes)
    }
}
