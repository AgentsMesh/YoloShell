use anyhow::{Context, Result, ensure};
use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use yolo_release_contracts::{
    BAZEL_OUTPUT_LINK_CHANGED_ERROR, BAZEL_OUTPUT_LINK_INSPECTION_ERROR,
    BAZEL_OUTPUT_LINK_LENGTH_ERROR, BAZEL_OUTPUT_LINK_READ_ERROR, BAZEL_OUTPUT_LINK_TYPE_ERROR,
    MAX_BAZEL_OUTPUT_LINK_BYTES,
};
use yolo_release_filesystem::DirectoryHandle;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LinkSnapshot {
    seal: LinkSeal,
    pub target: PathBuf,
}

impl LinkSnapshot {
    pub fn capture(parent: &DirectoryHandle, name: &str) -> Result<Self> {
        let name = CString::new(name)?;
        let before = LinkSeal::capture(parent, &name)?;
        let mut bytes = vec![0_u8; MAX_BAZEL_OUTPUT_LINK_BYTES];
        let count = unsafe {
            libc::readlinkat(
                parent.raw_fd(),
                name.as_ptr(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
            )
        };
        if count < 0 {
            return Err(std::io::Error::last_os_error()).context(BAZEL_OUTPUT_LINK_READ_ERROR);
        }
        let count = usize::try_from(count)?;
        ensure!(count < bytes.len(), BAZEL_OUTPUT_LINK_LENGTH_ERROR);
        bytes.truncate(count);
        let after = LinkSeal::capture(parent, &name)?;
        ensure!(before == after, BAZEL_OUTPUT_LINK_CHANGED_ERROR);
        Ok(Self {
            seal: before,
            target: PathBuf::from(OsString::from_vec(bytes)),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LinkSeal {
    device: u64,
    inode: u64,
    mode: u32,
    length: i64,
}

impl LinkSeal {
    fn capture(parent: &DirectoryHandle, name: &CString) -> Result<Self> {
        let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                parent.raw_fd(),
                name.as_ptr(),
                status.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error())
                .context(BAZEL_OUTPUT_LINK_INSPECTION_ERROR);
        }
        let status = unsafe { status.assume_init() };
        ensure!(
            status.st_mode & libc::S_IFMT == libc::S_IFLNK,
            BAZEL_OUTPUT_LINK_TYPE_ERROR
        );
        Ok(Self {
            device: status.st_dev as u64,
            inode: status.st_ino as u64,
            mode: status.st_mode as u32,
            length: status.st_size as i64,
        })
    }
}
