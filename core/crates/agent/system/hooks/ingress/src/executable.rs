use anyhow::{Result, ensure};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const INVALID: &str = "provider hook executable identity changed";
const EXECUTABLE_MASK: u32 = 0o111;

#[derive(Debug)]
pub struct HookExecutable {
    path: PathBuf,
    file: File,
    identity: yolo_paths::identity::ObjectIdentity,
    version: (u64, i64, i64, i64, i64),
}

impl HookExecutable {
    pub fn open(path: PathBuf) -> Result<Self> {
        ensure!(path.is_absolute() && path.canonicalize()? == path, INVALID);
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)?;
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file() && metadata.mode() & EXECUTABLE_MASK != u32::default(),
            INVALID
        );
        let authority = Self {
            path,
            file,
            identity: yolo_paths::identity::ObjectIdentity::from_metadata(&metadata),
            version: version(&metadata),
        };
        authority.verify()?;
        Ok(authority)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn verify(&self) -> Result<()> {
        let current = std::fs::symlink_metadata(&self.path)?;
        let retained = self.file.metadata()?;
        ensure!(
            current.is_file()
                && retained.is_file()
                && yolo_paths::identity::ObjectIdentity::from_metadata(&current) == self.identity
                && yolo_paths::identity::ObjectIdentity::from_metadata(&retained)
                    == self.identity
                && version(&current) == self.version
                && version(&retained) == self.version,
            INVALID
        );
        Ok(())
    }

    pub fn verify_process(&self, pid: u32) -> Result<()> {
        self.verify()?;
        #[cfg(target_os = "macos")]
        {
            use std::os::fd::AsRawFd;
            yolo_process::macos_mapped_executable::verify_mapped_executable(
                pid,
                self.file.as_raw_fd(),
            )?;
        }
        #[cfg(target_os = "linux")]
        {
            let mapped = yolo_process::linux_native::open_executable(pid)?;
            let metadata = mapped.metadata()?;
            ensure!(
                yolo_paths::identity::ObjectIdentity::from_metadata(&metadata) == self.identity
                    && version(&metadata) == self.version,
                INVALID
            );
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = pid;
            anyhow::bail!(INVALID);
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        self.verify()
    }
}

fn version(metadata: &std::fs::Metadata) -> (u64, i64, i64, i64, i64) {
    (
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}
