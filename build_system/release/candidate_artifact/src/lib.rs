mod io;

use crate::io::{copy, create, reopen};
use anyhow::{Context, Result, ensure};
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::Arc;
use yolo_release_contracts::{
    ARCHIVE_NAME, ARTIFACT_CHANGED_ERROR, ARTIFACT_PREFIX, MAX_ARTIFACT_BYTES, READ_ONLY_FILE_MODE,
};
use yolo_release_filesystem::{DirectoryHandle, DirectorySeal, TemporaryDirectory};
use yolo_release_foundation::digest::opened_file_sha256;
use yolo_release_foundation::file_seal::FileSeal;

const NAME_ERROR: &str = "candidate artifact has an unexpected name";
const OPEN_ERROR: &str = "open candidate artifact";
const TYPE_ERROR: &str = "candidate artifact must be a file";
const SIZE_ERROR: &str = "candidate artifact exceeded the bounded limit";
const PROTECT_ERROR: &str = "protect pinned candidate artifact";
const UNSEALED_ERROR: &str = "pinned candidate custody is not sealed";

pub struct PinnedArtifact {
    _root_owner: TemporaryDirectory,
    root: DirectoryHandle,
    root_seal: Option<DirectorySeal>,
    file: Arc<File>,
    file_seal: FileSeal,
    sha256: String,
    size_bytes: u64,
}

impl PinnedArtifact {
    pub fn open(source_path: &Path) -> Result<Self> {
        ensure!(
            source_path.file_name().and_then(|name| name.to_str()) == Some(ARCHIVE_NAME),
            NAME_ERROR
        );
        let source = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(source_path)
            .with_context(|| format!("{OPEN_ERROR} {}", source_path.display()))?;
        Self::from_opened(&source)
    }

    pub fn from_opened(source: &File) -> Result<Self> {
        Self::from_opened_with(source, |_| Ok(()))
    }

    fn from_opened_with(
        source: &File,
        after_seal: impl FnOnce(&Self) -> Result<()>,
    ) -> Result<Self> {
        let metadata = source.metadata()?;
        ensure!(metadata.is_file(), TYPE_ERROR);
        ensure!(metadata.len() <= MAX_ARTIFACT_BYTES, SIZE_ERROR);
        let source_seal = FileSeal::capture(source)?;
        let source_before = opened_file_sha256(source)?;
        source_seal.verify(source, ARTIFACT_CHANGED_ERROR)?;
        let root_owner = TemporaryDirectory::create(ARTIFACT_PREFIX)?;
        let root = root_owner.handle().clone();
        let mut writable = create(&root)?;
        copy(source, &mut writable, source_seal.length())?;
        source_seal.verify(source, ARTIFACT_CHANGED_ERROR)?;
        writable.sync_all()?;
        let status =
            unsafe { libc::fchmod(writable.as_raw_fd(), READ_ONLY_FILE_MODE as libc::mode_t) };
        ensure!(status == 0, PROTECT_ERROR);
        writable.sync_all()?;
        drop(writable);
        let file = Arc::new(reopen(&root)?);
        let file_seal = FileSeal::capture(&file)?;
        let sha256 = opened_file_sha256(&file)?;
        let source_after = opened_file_sha256(source)?;
        source_seal.verify(source, ARTIFACT_CHANGED_ERROR)?;
        ensure!(
            source_before == sha256 && sha256 == source_after,
            ARTIFACT_CHANGED_ERROR
        );
        let mut artifact = Self {
            _root_owner: root_owner,
            root,
            root_seal: None,
            file,
            file_seal,
            sha256,
            size_bytes: source_seal.length(),
        };
        artifact.root_seal = Some(artifact._root_owner.seal()?);
        after_seal(&artifact)?;
        artifact.verify()?;
        Ok(artifact)
    }

    fn verify(&self) -> Result<()> {
        let seal = self.root_seal.as_ref().context(UNSEALED_ERROR)?;
        self.root.verify_seal(seal)?;
        self.file_seal.verify(&self.file, ARTIFACT_CHANGED_ERROR)?;
        self.file_seal
            .verify(&reopen(&self.root)?, ARTIFACT_CHANGED_ERROR)?;
        ensure!(
            opened_file_sha256(&self.file)? == self.sha256,
            ARTIFACT_CHANGED_ERROR
        );
        Ok(())
    }

    pub fn sha256(&self) -> Result<String> {
        self.verify()?;
        Ok(self.sha256.clone())
    }

    pub fn size_bytes(&self) -> Result<u64> {
        self.verify()?;
        ensure!(
            self.file_seal.length() == self.size_bytes,
            ARTIFACT_CHANGED_ERROR
        );
        Ok(self.size_bytes)
    }

    pub fn copy_to(&self, destination: &mut File) -> Result<()> {
        self.verify()?;
        copy(&self.file, destination, self.size_bytes)?;
        self.verify()
    }

    pub fn independent_reader(&self) -> Result<Arc<File>> {
        self.verify()?;
        let reader = Arc::new(reopen(&self.root)?);
        self.file_seal.verify(&reader, ARTIFACT_CHANGED_ERROR)?;
        Ok(reader)
    }

    pub fn resource_directory(&self) -> &DirectoryHandle {
        &self.root
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub fn fixture(root: &Path, bytes: &[u8]) -> Result<Self> {
        let path = root.join(ARCHIVE_NAME);
        fs::write(&path, bytes)?;
        Self::open(&path)
    }
}

pub use io::copy as copy_opened;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/mod.rs"]
mod integration_tests;
