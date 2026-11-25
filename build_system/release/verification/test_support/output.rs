use crate::output::{OutputDirectory, OutputTransaction};
use crate::output_publish::PublishHooks;
use crate::validation::checksum_contents;
use std::fs;
use std::path::{Path, PathBuf};
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{
    NO_FILE_FLAGS, OUTPUT_TEMPORARY_PREFIX, PRIVATE_DIRECTORY_MODE, SEALED_DIRECTORY_MODE,
};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_foundation::json::encode;
use yolo_release_test_support::{ARCHIVE_BYTES, provenance};

pub(crate) struct OutputCleanup {
    path: PathBuf,
}

impl OutputCleanup {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
        }
    }
}

impl Drop for OutputCleanup {
    fn drop(&mut self) {
        if self.path.is_dir() {
            let _ = unseal(&self.path);
        }
    }
}

pub(crate) fn evidence() -> (Vec<u8>, Vec<u8>) {
    let digest = bytes_sha256(ARCHIVE_BYTES);
    let value = provenance(&digest, ARCHIVE_BYTES.len() as u64);
    (
        checksum_contents(&digest).into_bytes(),
        encode(&value).unwrap(),
    )
}

pub(crate) fn publish(root: &Path, path: &Path) -> OutputDirectory {
    let artifact = PinnedArtifact::fixture(root, ARCHIVE_BYTES).unwrap();
    let (checksum, provenance) = evidence();
    OutputTransaction::prepare(path)
        .unwrap()
        .publish(&artifact, &checksum, &provenance)
        .unwrap()
}

pub(crate) fn publish_with_hooks(
    root: &Path,
    path: &Path,
    hooks: &dyn PublishHooks,
) -> anyhow::Result<OutputDirectory> {
    let artifact = PinnedArtifact::fixture(root, ARCHIVE_BYTES)?;
    let (checksum, provenance) = evidence();
    OutputTransaction::prepare(path)?.publish_with_hooks(&artifact, &checksum, &provenance, hooks)
}

pub(crate) fn assert_no_staging_name(root: &Path) {
    assert!(fs::read_dir(root).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(OUTPUT_TEMPORARY_PREFIX)
    }));
}

pub fn unseal(path: &Path) -> anyhow::Result<DirectoryHandle> {
    let directory = DirectoryHandle::open(path)?;
    directory.set_flags(NO_FILE_FLAGS)?;
    directory.set_mode(PRIVATE_DIRECTORY_MODE)?;
    Ok(directory)
}

pub(crate) fn seal(path: &Path) -> anyhow::Result<DirectoryHandle> {
    let directory = DirectoryHandle::open(path)?;
    directory.set_mode(SEALED_DIRECTORY_MODE)?;
    directory.set_flags(libc::UF_IMMUTABLE)?;
    directory.durability_barrier()?;
    Ok(directory)
}
