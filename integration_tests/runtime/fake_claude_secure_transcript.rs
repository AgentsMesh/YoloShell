use serde::{Deserialize, Serialize};
use std::ffi::{CString, OsStr};
use std::fs::{File, Metadata};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

const DIRECTORY_FLAGS: libc::c_int =
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
const FILE_FLAGS: libc::c_int =
    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
const PATH_INVALID: &str = "Claude transcript path is outside its data home";
const OBJECT_INVALID: &str = "Claude transcript object is not trustworthy";
const EXPECTED_LINK_COUNT: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TranscriptIdentity {
    pub(crate) path: PathBuf,
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

impl TranscriptIdentity {
    pub(crate) fn capture(data_home: &Path, path: &Path) -> io::Result<Self> {
        let relative = path.strip_prefix(data_home).map_err(|_| invalid_path())?;
        let names = normal_components(relative)?;
        let (file_name, directories) = names.split_last().ok_or_else(invalid_path)?;
        let mut directory = open_path(data_home, DIRECTORY_FLAGS)?;
        verify_directory(&directory.metadata()?)?;
        for name in directories {
            directory = open_at(directory.as_raw_fd(), name, DIRECTORY_FLAGS)?;
            verify_directory(&directory.metadata()?)?;
        }
        let file = open_at(directory.as_raw_fd(), file_name, FILE_FLAGS)?;
        let metadata = file.metadata()?;
        verify_file(&metadata)?;
        let current = open_at(directory.as_raw_fd(), file_name, FILE_FLAGS)?;
        let current_metadata = current.metadata()?;
        verify_file(&current_metadata)?;
        if object_key(&metadata) != object_key(&current_metadata) {
            return Err(io::Error::other(OBJECT_INVALID));
        }
        Ok(Self {
            path: path.to_path_buf(),
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub(crate) fn still_matches(&self, data_home: &Path) -> bool {
        Self::capture(data_home, &self.path).ok().as_ref() == Some(self)
    }
}

fn normal_components(path: &Path) -> io::Result<Vec<&OsStr>> {
    path.components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err(invalid_path()),
        })
        .collect()
}

fn open_path(path: &Path, flags: libc::c_int) -> io::Result<File> {
    let name = CString::new(path.as_os_str().as_bytes()).map_err(|_| invalid_path())?;
    owned_fd(unsafe { libc::open(name.as_ptr(), flags) })
}

fn open_at(parent: RawFd, name: &OsStr, flags: libc::c_int) -> io::Result<File> {
    let name = CString::new(name.as_bytes()).map_err(|_| invalid_path())?;
    owned_fd(unsafe { libc::openat(parent, name.as_ptr(), flags) })
}

fn owned_fd(fd: RawFd) -> io::Result<File> {
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

fn verify_directory(metadata: &Metadata) -> io::Result<()> {
    if metadata.is_dir() && metadata.uid() == unsafe { libc::geteuid() } {
        Ok(())
    } else {
        Err(io::Error::other(OBJECT_INVALID))
    }
}

fn verify_file(metadata: &Metadata) -> io::Result<()> {
    if metadata.is_file()
        && metadata.nlink() == EXPECTED_LINK_COUNT
        && metadata.uid() == unsafe { libc::geteuid() }
    {
        Ok(())
    } else {
        Err(io::Error::other(OBJECT_INVALID))
    }
}

fn object_key(metadata: &Metadata) -> (u64, u64) {
    (metadata.dev(), metadata.ino())
}

fn invalid_path() -> io::Error {
    io::Error::other(PATH_INVALID)
}
