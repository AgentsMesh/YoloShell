use anyhow::{Context, Result};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::FromRawFd;
use yolo_release_contracts::{
    NO_FILE_FLAGS, PRIVATE_DIRECTORY_MODE, PRIVATE_FILE_MODE, READ_ONLY_FILE_MODE,
    RUST_COVERAGE_CUSTODY_DIRECTORY_NAME, RUST_COVERAGE_CUSTODY_FILE_NAME,
    RUST_COVERAGE_DIRECTORY_CREATE_ERROR, RUST_COVERAGE_FILE_CREATE_ERROR,
    RUST_COVERAGE_FILE_OPEN_ERROR,
};
use yolo_release_filesystem::DirectoryHandle;

const SYSCALL_SUCCESS: libc::c_int = 0;
const OPEN_ERROR: libc::c_int = -1;
const FILE_UNLINK_FLAGS: libc::c_int = 0;

pub(crate) struct CoverageDirectory {
    parent: DirectoryHandle,
    handle: DirectoryHandle,
    path: std::path::PathBuf,
}

impl CoverageDirectory {
    pub fn create(parent: &DirectoryHandle) -> Result<Self> {
        parent.verify_path()?;
        let name = CString::new(RUST_COVERAGE_CUSTODY_DIRECTORY_NAME)?;
        let status = unsafe {
            libc::mkdirat(
                parent.raw_fd(),
                name.as_ptr(),
                PRIVATE_DIRECTORY_MODE as libc::mode_t,
            )
        };
        if status != SYSCALL_SUCCESS {
            return Err(std::io::Error::last_os_error())
                .context(RUST_COVERAGE_DIRECTORY_CREATE_ERROR);
        }
        let handle = DirectoryHandle::open_at(parent, RUST_COVERAGE_CUSTODY_DIRECTORY_NAME)?;
        let custody = Self {
            parent: parent.clone(),
            path: handle.display_path().join(RUST_COVERAGE_CUSTODY_FILE_NAME),
            handle,
        };
        custody
            .parent
            .verify_child(RUST_COVERAGE_CUSTODY_DIRECTORY_NAME, &custody.handle)?;
        custody.handle.set_mode(PRIVATE_DIRECTORY_MODE)?;
        Ok(custody)
    }

    pub fn handle(&self) -> &DirectoryHandle {
        &self.handle
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for CoverageDirectory {
    fn drop(&mut self) {
        let _ = self.handle.set_flags(NO_FILE_FLAGS);
        let _ = self.handle.set_mode(PRIVATE_DIRECTORY_MODE);
        unlink(
            self.handle.raw_fd(),
            RUST_COVERAGE_CUSTODY_FILE_NAME,
            FILE_UNLINK_FLAGS,
        );
        if self
            .parent
            .verify_child(RUST_COVERAGE_CUSTODY_DIRECTORY_NAME, &self.handle)
            .is_ok()
        {
            unlink(
                self.parent.raw_fd(),
                RUST_COVERAGE_CUSTODY_DIRECTORY_NAME,
                libc::AT_REMOVEDIR,
            );
        }
    }
}

pub fn create_file(root: &DirectoryHandle) -> Result<File> {
    open_file(
        root,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        PRIVATE_FILE_MODE,
        RUST_COVERAGE_FILE_CREATE_ERROR,
    )
}

pub fn reopen_file(root: &DirectoryHandle) -> Result<File> {
    open_file(
        root,
        libc::O_RDONLY | libc::O_NONBLOCK,
        READ_ONLY_FILE_MODE,
        RUST_COVERAGE_FILE_OPEN_ERROR,
    )
}

fn open_file(root: &DirectoryHandle, flags: libc::c_int, mode: u32, context: &str) -> Result<File> {
    let name = CString::new(RUST_COVERAGE_CUSTODY_FILE_NAME)?;
    let descriptor = unsafe {
        libc::openat(
            root.raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            mode as libc::c_uint,
        )
    };
    if descriptor == OPEN_ERROR {
        return Err(std::io::Error::last_os_error()).context(context.to_owned());
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn unlink(parent: libc::c_int, name: &str, flags: libc::c_int) {
    if let Ok(name) = CString::new(name) {
        unsafe { libc::unlinkat(parent, name.as_ptr(), flags) };
    }
}
