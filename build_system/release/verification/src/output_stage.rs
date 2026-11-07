use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{
    ARCHIVE_NAME, CHECKSUM_NAME, NO_FILE_FLAGS, OUTPUT_TEMPORARY_PREFIX, PRIVATE_DIRECTORY_MODE,
    PRIVATE_FILE_MODE, PROVENANCE_NAME, RANDOM_DEVICE, RANDOM_SUFFIX_BYTES, READ_ONLY_FILE_MODE,
};
use yolo_release_filesystem::DirectoryHandle;

pub(crate) struct StagedCandidate {
    pub(crate) parent: DirectoryHandle,
    pub(crate) handle: DirectoryHandle,
    pub(crate) name: CString,
    pub(crate) installed: bool,
    pub(crate) committed: bool,
}

impl StagedCandidate {
    pub fn create(parent: &DirectoryHandle) -> Result<Self> {
        let name = temporary_name()?;
        let status = unsafe {
            libc::mkdirat(
                parent.raw_fd(),
                name.as_ptr(),
                PRIVATE_DIRECTORY_MODE as libc::mode_t,
            )
        };
        if status != 0 {
            return Err(std::io::Error::last_os_error()).context("stage candidate directory");
        }
        let display_name = name.to_str()?.to_owned();
        let handle = DirectoryHandle::open_at(parent, &display_name)?;
        Ok(Self {
            parent: parent.clone(),
            handle,
            name,
            installed: false,
            committed: false,
        })
    }

    pub fn handle(&self) -> &DirectoryHandle {
        &self.handle
    }

    pub fn write(
        &mut self,
        artifact: &PinnedArtifact,
        checksum: &[u8],
        provenance: &[u8],
    ) -> Result<()> {
        self.write_file(ARCHIVE_NAME, |file| artifact.copy_to(file))?;
        self.write_file(CHECKSUM_NAME, |file| write_bytes(file, checksum))?;
        self.write_file(PROVENANCE_NAME, |file| write_bytes(file, provenance))?;
        self.handle.sync_all()
    }

    fn write_file<F>(&self, name: &str, writer: F) -> Result<()>
    where
        F: FnOnce(&mut File) -> Result<()>,
    {
        let name = CString::new(name)?;
        let descriptor = unsafe {
            libc::openat(
                self.handle.raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                PRIVATE_FILE_MODE as libc::c_uint,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error()).context("write staged candidate file");
        }
        let mut file = unsafe { File::from_raw_fd(descriptor) };
        writer(&mut file)?;
        file.sync_all()?;
        let status = unsafe { libc::fchmod(file.as_raw_fd(), READ_ONLY_FILE_MODE as libc::mode_t) };
        ensure!(status == 0, "protect staged candidate file");
        file.sync_all().context("sync protected candidate file")
    }
}

impl Drop for StagedCandidate {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let _ = self.handle.set_flags(NO_FILE_FLAGS);
        let _ = self.handle.set_mode(PRIVATE_DIRECTORY_MODE);
        for name in [ARCHIVE_NAME, CHECKSUM_NAME, PROVENANCE_NAME] {
            if let Ok(name) = CString::new(name) {
                unsafe { libc::unlinkat(self.handle.raw_fd(), name.as_ptr(), 0) };
            }
        }
        if !self.installed
            && self
                .name
                .to_str()
                .ok()
                .is_some_and(|name| self.parent.verify_child(name, &self.handle).is_ok())
        {
            unsafe { libc::unlinkat(self.parent.raw_fd(), self.name.as_ptr(), libc::AT_REMOVEDIR) };
        }
    }
}

fn temporary_name() -> Result<CString> {
    let mut random = [0_u8; RANDOM_SUFFIX_BYTES];
    File::open(RANDOM_DEVICE)?.read_exact(&mut random)?;
    let suffix = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    CString::new(format!("{OUTPUT_TEMPORARY_PREFIX}{suffix}")).map_err(Into::into)
}

fn write_bytes(file: &mut File, bytes: &[u8]) -> Result<()> {
    file.write_all(bytes)
        .context("write staged candidate output")
}
