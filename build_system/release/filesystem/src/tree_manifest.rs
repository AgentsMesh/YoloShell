use crate::constants::{
    CURRENT_DIRECTORY_COMPONENT, PARENT_DIRECTORY_COMPONENT, TREE_MANIFEST_DIRECTORY_MARKER,
    TREE_MANIFEST_FIELD_SEPARATOR, TREE_MANIFEST_FILE_MARKER, WRITE_PERMISSION_MASK,
};
use crate::directory::DirectoryHandle;
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::ffi::{CStr, CString, OsString};
use std::fs::{self, File};
use std::os::fd::FromRawFd;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use yolo_release_foundation::digest::{hex_digest, opened_file_sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeManifest {
    sha256: String,
}

impl TreeManifest {
    pub fn capture(root: &DirectoryHandle) -> Result<Self> {
        root.verify_self()?;
        let mut digest = Sha256::new();
        record_directory(&mut digest, &PathBuf::new(), root)?;
        Ok(Self {
            sha256: hex_digest(digest.finalize()),
        })
    }
}

fn record_directory(
    digest: &mut Sha256,
    relative: &Path,
    directory: &DirectoryHandle,
) -> Result<()> {
    record_metadata(
        digest,
        relative,
        TREE_MANIFEST_DIRECTORY_MARKER,
        &directory.retained_file().metadata()?,
    )?;
    for name in sorted_names(directory)? {
        let path = relative.join(&name);
        let file = open_child(directory, &name)?;
        let metadata = file.metadata()?;
        if metadata.is_dir() {
            let child_name = name
                .to_str()
                .context("snapshot directory name must be UTF-8")?;
            let child = DirectoryHandle::open_at(directory, child_name)?;
            ensure!(
                child.identity().device == metadata.dev()
                    && child.identity().inode == metadata.ino(),
                "snapshot directory changed while manifest was captured"
            );
            record_directory(digest, &path, &child)?;
        } else {
            ensure!(metadata.is_file(), "snapshot contains a special file");
            record_metadata(digest, &path, TREE_MANIFEST_FILE_MARKER, &metadata)?;
            update(digest, opened_file_sha256(&file)?.as_bytes());
        }
    }
    Ok(())
}

fn record_metadata(
    digest: &mut Sha256,
    relative: &Path,
    marker: &[u8],
    metadata: &fs::Metadata,
) -> Result<()> {
    ensure!(
        metadata.mode() & WRITE_PERMISSION_MASK == 0,
        "snapshot source regained write permission"
    );
    update(digest, marker);
    update(digest, relative.as_os_str().as_bytes());
    for value in [
        metadata.dev(),
        metadata.ino(),
        u64::from(metadata.mode()),
        u64::from(metadata.uid()),
        u64::from(metadata.gid()),
        metadata.nlink(),
        metadata.size(),
        signed(metadata.mtime()),
        signed(metadata.mtime_nsec()),
        signed(metadata.ctime()),
        signed(metadata.ctime_nsec()),
    ] {
        update(digest, value.to_string().as_bytes());
    }
    Ok(())
}

pub(crate) fn sorted_names(directory: &DirectoryHandle) -> Result<Vec<OsString>> {
    let current = CString::new(CURRENT_DIRECTORY_COMPONENT)?;
    let descriptor = unsafe {
        libc::openat(
            directory.raw_fd(),
            current.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("duplicate manifest directory");
    }
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe { libc::close(descriptor) };
        return Err(std::io::Error::last_os_error()).context("open manifest directory stream");
    }
    let stream = DirectoryStream(stream);
    let mut names = Vec::new();
    loop {
        unsafe { *libc::__error() = 0 };
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(0) {
                break;
            }
            return Err(error).context("read manifest directory stream");
        }
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes != CURRENT_DIRECTORY_COMPONENT.as_bytes()
            && bytes != PARENT_DIRECTORY_COMPONENT.as_bytes()
        {
            names.push(OsString::from_vec(bytes.to_vec()));
        }
    }
    names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    Ok(names)
}

struct DirectoryStream(*mut libc::DIR);

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}

fn open_child(directory: &DirectoryHandle, name: &OsString) -> Result<File> {
    let name = CString::new(name.as_bytes()).context("snapshot name contains NUL")?;
    let descriptor = unsafe {
        libc::openat(
            directory.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("open snapshot manifest entry");
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn update(digest: &mut Sha256, value: &[u8]) {
    digest.update(value);
    digest.update(TREE_MANIFEST_FIELD_SEPARATOR);
}

fn signed(value: i64) -> u64 {
    u64::from_ne_bytes(value.to_ne_bytes())
}
