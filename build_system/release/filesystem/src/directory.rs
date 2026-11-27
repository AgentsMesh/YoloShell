use crate::constants::{DIRECTORY_DURABILITY_ERROR, DURABILITY_BARRIER_SUCCESS};
use anyhow::{Context, Result, ensure};
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use yolo_release_evidence_model::DirectoryIdentityEvidence;

const OPEN_ERROR: libc::c_int = -1;

#[derive(Clone, Debug)]
pub struct DirectoryHandle {
    file: Arc<File>,
    identity: DirectoryIdentityEvidence,
    display_path: PathBuf,
    pub(crate) resources: Vec<crate::directory_resources::DirectoryResources>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectorySeal {
    device: u64,
    inode: u64,
    mode: u32,
    change_seconds: i64,
    change_nanoseconds: i64,
    flags: u32,
}

impl DirectoryHandle {
    pub fn open(path: &Path) -> Result<Self> {
        let name = cstring(path)?;
        let descriptor = unsafe { libc::open(name.as_ptr(), directory_flags()) };
        Self::from_descriptor(descriptor, path.to_owned())
    }

    pub fn open_at(parent: &Self, name: &str) -> Result<Self> {
        Self::open_at_bytes(parent, name.as_bytes())
    }

    pub fn open_at_bytes(parent: &Self, name: &[u8]) -> Result<Self> {
        ensure!(
            !name.is_empty() && !name.contains(&b'/'),
            "retained child name must be one path component"
        );
        let child_name = CString::new(name)?;
        let descriptor =
            unsafe { libc::openat(parent.raw_fd(), child_name.as_ptr(), directory_flags()) };
        let mut child = Self::from_descriptor(
            descriptor,
            parent.display_path.join(OsStr::from_bytes(name)),
        )?;
        child.resources = parent.resources.clone();
        Ok(child)
    }

    fn from_descriptor(descriptor: RawFd, display_path: PathBuf) -> Result<Self> {
        if descriptor == OPEN_ERROR {
            return Err(std::io::Error::last_os_error()).context("open retained directory");
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let metadata = file.metadata()?;
        ensure!(metadata.is_dir(), "retained path must be a directory");
        Ok(Self {
            identity: DirectoryIdentityEvidence {
                device: metadata.dev(),
                inode: metadata.ino(),
            },
            file: Arc::new(file),
            display_path,
            resources: Vec::new(),
        })
    }

    pub fn verify_self(&self) -> Result<()> {
        let metadata = self.file.metadata()?;
        ensure!(
            metadata.dev() == self.identity.device && metadata.ino() == self.identity.inode,
            "retained directory identity changed"
        );
        Ok(())
    }

    pub fn verify_path(&self) -> Result<()> {
        self.verify_self()?;
        let reopened = Self::open(&self.display_path)?;
        ensure!(
            reopened.identity == self.identity,
            "retained directory path identity changed"
        );
        Ok(())
    }

    pub fn verify_child(&self, name: &str, expected: &Self) -> Result<()> {
        self.verify_child_bytes(name.as_bytes(), expected)
    }

    pub fn verify_child_bytes(&self, name: &[u8], expected: &Self) -> Result<()> {
        self.verify_self()?;
        let current = Self::open_at_bytes(self, name)?;
        ensure!(
            current.identity == expected.identity,
            "snapshot child directory identity changed"
        );
        expected.verify_self()
    }

    pub fn raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    pub fn identity(&self) -> DirectoryIdentityEvidence {
        self.identity.clone()
    }

    pub fn display_path(&self) -> &Path {
        &self.display_path
    }

    pub fn retained_file(&self) -> Arc<File> {
        Arc::clone(&self.file)
    }

    pub fn mode(&self) -> Result<u32> {
        Ok(self.file.metadata()?.permissions().mode())
    }

    pub fn sync_all(&self) -> Result<()> {
        self.file.sync_all().context("sync retained directory")
    }

    pub fn durability_barrier(&self) -> Result<()> {
        self.verify_self()?;
        let status = unsafe { libc::fcntl(self.raw_fd(), libc::F_FULLFSYNC) };
        if status != DURABILITY_BARRIER_SUCCESS {
            return Err(std::io::Error::last_os_error()).context(DIRECTORY_DURABILITY_ERROR);
        }
        self.verify_self()
    }

    pub fn lock_exclusive(&self) -> Result<crate::directory_lock::DirectoryLock> {
        crate::directory_lock::lock_exclusive(self)
    }

    pub fn set_mode(&self, mode: u32) -> Result<()> {
        let status = unsafe { libc::fchmod(self.raw_fd(), mode as libc::mode_t) };
        ensure!(status == 0, "set retained directory mode");
        Ok(())
    }

    pub fn set_flags(&self, flags: u32) -> Result<()> {
        let status = unsafe { libc::fchflags(self.raw_fd(), flags as libc::c_uint) };
        ensure!(status == 0, "set retained directory flags");
        Ok(())
    }

    pub fn flags(&self) -> Result<u32> {
        file_flags(self.raw_fd())
    }

    pub fn seal(&self) -> Result<DirectorySeal> {
        let metadata = self.file.metadata()?;
        Ok(DirectorySeal {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.permissions().mode(),
            change_seconds: metadata.ctime(),
            change_nanoseconds: metadata.ctime_nsec(),
            flags: file_flags(self.raw_fd())?,
        })
    }

    pub fn verify_seal(&self, expected: &DirectorySeal) -> Result<()> {
        ensure!(self.seal()? == *expected, "sealed directory changed");
        self.verify_path()
    }
}

fn file_flags(descriptor: RawFd) -> Result<u32> {
    let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
    ensure!(
        unsafe { libc::fstat(descriptor, status.as_mut_ptr()) } == 0,
        "inspect retained directory flags"
    );
    Ok(unsafe { status.assume_init() }.st_flags)
}

pub(crate) fn directory_flags() -> libc::c_int {
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC
}

fn cstring(path: &Path) -> Result<CString> {
    CString::new(path.as_os_str().as_bytes()).context("directory path contains NUL")
}
