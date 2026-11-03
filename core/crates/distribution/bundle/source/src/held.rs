use crate::constants::*;
use anyhow::{Result, ensure};
use std::{
    fs::{File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use yolo_paths::identity::ObjectIdentity;

#[derive(Eq, PartialEq)]
struct Seal {
    object: ObjectIdentity,
    length: u64,
    mode: u32,
    user: u32,
    links: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
pub(crate) struct Held {
    pub file: File,
    path: PathBuf,
    seal: Seal,
    directory: bool,
    maximum: u64,
}
impl Held {
    pub fn open(path: &Path, directory: bool, maximum: u64) -> Result<Self> {
        ensure!(
            path.is_absolute() && std::fs::canonicalize(path)? == path,
            INVALID
        );
        let file = open(path, directory)?;
        let seal = capture(&file, path, directory, maximum)?;
        let held = Self {
            file,
            path: path.into(),
            seal,
            directory,
            maximum,
        };
        held.verify()?;
        Ok(held)
    }
    pub fn verify(&self) -> Result<()> {
        ensure!(std::fs::canonicalize(&self.path)? == self.path, CHANGED);
        ensure!(
            capture(&self.file, &self.path, self.directory, self.maximum)? == self.seal,
            CHANGED
        );
        let named = open(&self.path, self.directory)?;
        ensure!(
            capture(&named, &self.path, self.directory, self.maximum)? == self.seal,
            CHANGED
        );
        Ok(())
    }
    pub fn bytes(&self) -> Result<Vec<u8>> {
        use std::os::unix::fs::FileExt;
        self.verify()?;
        ensure!(!self.directory, INVALID);
        let mut bytes = vec![0; usize::try_from(self.seal.length)?];
        self.file.read_exact_at(&mut bytes, 0)?;
        self.verify()?;
        Ok(bytes)
    }
    pub fn executable(&self) -> bool {
        self.seal.mode & EXECUTE_ACCESS != 0
    }
}
fn open(path: &Path, directory: bool) -> std::io::Result<File> {
    let flags = libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    OpenOptions::new().read(true).custom_flags(flags).open(path)
}
fn capture(file: &File, path: &Path, directory: bool, maximum: u64) -> Result<Seal> {
    let metadata = file.metadata()?;
    ensure!(
        if directory {
            metadata.is_dir()
        } else {
            metadata.is_file()
                && metadata.nlink() == REGULAR_LINKS
                && metadata.len() > 0
                && metadata.len() <= maximum
        },
        INVALID
    );
    ensure!(
        metadata.mode() & WRITE_ACCESS == 0
            && (metadata.uid() == ROOT_USER || metadata.uid() == unsafe { libc::geteuid() }),
        INVALID
    );
    yolo_paths::private_acl::require_no_grants(file, path)?;
    let object = ObjectIdentity::from_file(file)?.ok_or_else(|| anyhow::anyhow!(INVALID))?;
    Ok(Seal {
        object,
        length: metadata.len(),
        mode: metadata.mode(),
        user: metadata.uid(),
        links: metadata.nlink(),
        modified: (metadata.mtime(), metadata.mtime_nsec()),
        changed: (metadata.ctime(), metadata.ctime_nsec()),
    })
}
