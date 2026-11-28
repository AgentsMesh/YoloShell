use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::{File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

const PRIVATE_FILE_MODE: u32 = 0o600;
const PERMISSION_BITS_MASK: u32 = 0o7777;
const LOCK_MARKER: &[u8] = b"Yolo remote host authority v1\n";
const MAXIMUM_LOCK_BYTES: u64 = 4 * 1024;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct FileIdentity {
    device: u64,
    inode: u64,
}

pub(super) struct AuthorityLock {
    file: File,
    identity: FileIdentity,
    path: PathBuf,
}

impl AuthorityLock {
    pub(super) fn acquire(path: &Path) -> Result<Self> {
        let file = open_file(path, libc::O_RDWR, MAXIMUM_LOCK_BYTES)?;
        lock_exclusive(&file)?;
        let identity = inspect(&file.metadata()?, path, MAXIMUM_LOCK_BYTES)?;
        verify_named(path, identity, MAXIMUM_LOCK_BYTES)?;
        ensure!(
            read_bounded(&file, MAXIMUM_LOCK_BYTES)? == LOCK_MARKER,
            "invalid authority lock marker"
        );
        Ok(Self {
            file,
            identity,
            path: path.to_owned(),
        })
    }

    pub(super) fn verify(&self) -> Result<()> {
        let identity = inspect(&self.file.metadata()?, &self.path, MAXIMUM_LOCK_BYTES)?;
        ensure!(identity == self.identity, "authority lock identity changed");
        verify_named(&self.path, self.identity, MAXIMUM_LOCK_BYTES)?;
        ensure!(
            read_bounded(&self.file, MAXIMUM_LOCK_BYTES)? == LOCK_MARKER,
            "invalid authority lock marker"
        );
        Ok(())
    }
}

pub(super) fn read_document(path: &Path, maximum_bytes: u64) -> Result<(Vec<u8>, FileIdentity)> {
    let file = open_file(path, libc::O_RDONLY, maximum_bytes)?;
    let identity = inspect(&file.metadata()?, path, maximum_bytes)?;
    verify_named(path, identity, maximum_bytes)?;
    let bytes = read_bounded(&file, maximum_bytes)?;
    let held = inspect(&file.metadata()?, path, maximum_bytes)?;
    ensure!(
        held == identity,
        "authority document identity changed while reading"
    );
    verify_named(path, identity, maximum_bytes)?;
    Ok((bytes, identity))
}

fn open_file(path: &Path, access: libc::c_int, maximum_bytes: u64) -> Result<File> {
    validate_path(path)?;
    let encoded =
        CString::new(path.as_os_str().as_bytes()).context("authority path contains a null byte")?;
    let flags = access | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    let descriptor = unsafe { libc::open(encoded.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("open authority file {}", path.display()));
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    inspect(&file.metadata()?, path, maximum_bytes)?;
    Ok(file)
}

fn validate_path(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "authority path must be absolute");
    ensure!(
        path.components()
            .all(|part| matches!(part, Component::RootDir | Component::Normal(_))),
        "authority path must be lexically normalized"
    );
    Ok(())
}

fn inspect(metadata: &Metadata, path: &Path, maximum_bytes: u64) -> Result<FileIdentity> {
    ensure!(
        metadata.is_file(),
        "authority entry is not a regular file: {}",
        path.display()
    );
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "authority file owner mismatch: {}",
        path.display()
    );
    ensure!(
        metadata.mode() & PERMISSION_BITS_MASK == PRIVATE_FILE_MODE,
        "authority file mode mismatch: {}",
        path.display()
    );
    ensure!(
        metadata.nlink() == 1,
        "authority file link count mismatch: {}",
        path.display()
    );
    ensure!(
        metadata.len() <= maximum_bytes,
        "authority file exceeds byte limit: {}",
        path.display()
    );
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

fn verify_named(path: &Path, expected: FileIdentity, maximum_bytes: u64) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).context("inspect named authority file")?;
    ensure!(
        inspect(&metadata, path, maximum_bytes)? == expected,
        "authority file namespace changed"
    );
    Ok(())
}

fn read_bounded(file: &File, maximum_bytes: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut reader = file;
    reader.seek(SeekFrom::Start(0))?;
    reader
        .take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= maximum_bytes,
        "authority file exceeds byte limit"
    );
    Ok(bytes)
}

fn lock_exclusive(file: &File) -> Result<()> {
    loop {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error).context("lock remote host authority");
        }
    }
}
