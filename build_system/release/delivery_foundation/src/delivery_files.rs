use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use yolo_release_contracts::*;
use yolo_release_contracts::{
    EXPECTED_FILE_LINKS, PERMISSION_BITS_MASK, PRIVATE_FILE_MODE, READ_ONLY_FILE_MODE,
};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_filesystem::ensure_no_extended_acl;
use yolo_release_foundation::file_seal::FileSeal;

pub struct ProtectedFile {
    file: File,
    seal: FileSeal,
}

impl ProtectedFile {
    pub fn create<F>(root: &DirectoryHandle, name: &str, writer: F) -> Result<Self>
    where
        F: FnOnce(&mut File) -> Result<()>,
    {
        let mut file = create_exclusive(root, name)?;
        writer(&mut file)?;
        file.sync_all()?;
        ensure!(
            unsafe { libc::fchmod(file.as_raw_fd(), READ_ONLY_FILE_MODE as libc::mode_t) }
                == DELIVERY_SYSTEM_CALL_SUCCESS,
            DELIVERY_FILE_MODE_ERROR
        );
        file.sync_all()?;
        validate(&file)?;
        let seal = FileSeal::capture(&file)?;
        Ok(Self { file, seal })
    }

    pub fn open(root: &DirectoryHandle, name: &str) -> Result<Self> {
        let file = reopen(root, name)?;
        validate(&file)?;
        let seal = FileSeal::capture(&file)?;
        Ok(Self { file, seal })
    }

    pub fn seal_existing(root: &DirectoryHandle, name: &str) -> Result<Self> {
        let file = reopen_writable(root, name)?;
        ensure!(
            unsafe { libc::fchmod(file.as_raw_fd(), READ_ONLY_FILE_MODE as libc::mode_t) }
                == DELIVERY_SYSTEM_CALL_SUCCESS,
            DELIVERY_FILE_MODE_ERROR
        );
        file.sync_all()?;
        validate(&file)?;
        let seal = FileSeal::capture(&file)?;
        Ok(Self { file, seal })
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn verify_at(&self, root: &DirectoryHandle, name: &str) -> Result<File> {
        self.seal.verify(&self.file, DELIVERY_FILE_CHANGED_ERROR)?;
        let reopened = reopen(root, name)?;
        validate(&reopened)?;
        self.seal.verify(&reopened, DELIVERY_FILE_CHANGED_ERROR)?;
        Ok(reopened)
    }

    pub fn verify_contents(
        &self,
        root: &DirectoryHandle,
        name: &str,
        expected: &[u8],
    ) -> Result<()> {
        let reopened = self.verify_at(root, name)?;
        ensure!(
            reopened.metadata()?.len() == u64::try_from(expected.len())?,
            DELIVERY_CHECKSUM_MISMATCH_ERROR
        );
        let mut actual = vec![0_u8; expected.len()];
        reopened.read_exact_at(&mut actual, 0)?;
        ensure!(actual == expected, DELIVERY_CHECKSUM_MISMATCH_ERROR);
        self.seal.verify(&reopened, DELIVERY_FILE_CHANGED_ERROR)
    }

    pub fn bind_after_rename(
        mut self,
        root: &DirectoryHandle,
        name: &str,
        expected: &[u8],
    ) -> Result<Self> {
        self.seal = self
            .seal
            .recapture_after_rename(&self.file, DELIVERY_FILE_CHANGED_ERROR)?;
        self.verify_contents(root, name, expected)?;
        Ok(self)
    }
}

fn create_exclusive(root: &DirectoryHandle, name: &str) -> Result<File> {
    root.verify_self()?;
    let name = CString::new(name)?;
    let descriptor = unsafe {
        libc::openat(
            root.raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            PRIVATE_FILE_MODE as libc::c_uint,
        )
    };
    if descriptor == DELIVERY_OPEN_FAILURE {
        return Err(std::io::Error::last_os_error()).context(DELIVERY_FILE_CREATE_ERROR);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn reopen(root: &DirectoryHandle, name: &str) -> Result<File> {
    reopen_with_flags(root, name, libc::O_RDONLY | libc::O_NONBLOCK)
}

fn reopen_writable(root: &DirectoryHandle, name: &str) -> Result<File> {
    reopen_with_flags(root, name, libc::O_RDWR | libc::O_NONBLOCK)
}

fn reopen_with_flags(root: &DirectoryHandle, name: &str, access: libc::c_int) -> Result<File> {
    root.verify_self()?;
    let name = CString::new(name)?;
    let descriptor = unsafe {
        libc::openat(
            root.raw_fd(),
            name.as_ptr(),
            access | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor == DELIVERY_OPEN_FAILURE {
        return Err(std::io::Error::last_os_error()).context(DELIVERY_FILE_OPEN_ERROR);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn validate(file: &File) -> Result<()> {
    let metadata = file.metadata()?;
    ensure!(metadata.is_file(), DELIVERY_FILE_TYPE_ERROR);
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        DELIVERY_FILE_OWNER_ERROR
    );
    ensure!(
        metadata.permissions().mode() & PERMISSION_BITS_MASK == READ_ONLY_FILE_MODE,
        DELIVERY_FILE_MODE_ERROR
    );
    ensure!(
        metadata.nlink() == EXPECTED_FILE_LINKS,
        DELIVERY_FILE_LINK_ERROR
    );
    ensure_no_extended_acl(file.as_raw_fd(), DELIVERY_FILE_TYPE_ERROR)
}
