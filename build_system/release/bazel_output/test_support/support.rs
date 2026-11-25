use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use yolo_release_bazel_output::BazelOutputBinding;
use yolo_release_contracts::{
    BAZEL_BIN_LINK_NAME, LINKS_DIRECTORY_NAME, OUTPUT_DIRECTORY_NAME,
    PACKAGE_ARTIFACT_RELATIVE_PATH,
};
use yolo_release_filesystem::DirectoryHandle;

pub const ARCHIVE_BYTES: &[u8] = b"stable candidate archive\n";
pub const OTHER_ARCHIVE_BYTES: &[u8] = b"different candidate archive\n";
pub const BAZEL_OUTPUT_REVISION_A: &str = "revision-a";
pub const BAZEL_OUTPUT_REVISION_B: &str = "revision-b";
pub const BAZEL_OUTPUT_ORIGINAL_NAME: &str = "original-Yolo.zip";

pub struct OutputFixture {
    _root: TempDir,
    output: DirectoryHandle,
    links: DirectoryHandle,
}

impl OutputFixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join(OUTPUT_DIRECTORY_NAME);
        let links = root.path().join(LINKS_DIRECTORY_NAME);
        fs::create_dir(&output).unwrap();
        fs::create_dir(&links).unwrap();
        Self {
            output: DirectoryHandle::open(&output).unwrap(),
            links: DirectoryHandle::open(&links).unwrap(),
            _root: root,
        }
    }

    pub fn binding(&self) -> anyhow::Result<BazelOutputBinding> {
        BazelOutputBinding::capture(
            &self.output,
            &self.links,
            BAZEL_BIN_LINK_NAME,
            PACKAGE_ARTIFACT_RELATIVE_PATH,
        )
    }

    pub fn create_output(&self, revision: &str, bytes: &[u8]) -> PathBuf {
        let target = self.output.display_path().join(revision);
        create_archive(&target, bytes);
        target
    }

    pub fn point_link(&self, target: &Path) {
        let link = self.links.display_path().join(BAZEL_BIN_LINK_NAME);
        if link.exists() || link.is_symlink() {
            fs::remove_file(&link).unwrap();
        }
        symlink(target, link).unwrap();
    }
}

pub fn create_archive(target: &Path, bytes: &[u8]) {
    let archive = target.join(PACKAGE_ARTIFACT_RELATIVE_PATH);
    fs::create_dir_all(archive.parent().unwrap()).unwrap();
    fs::write(archive, bytes).unwrap();
}

pub fn error_text<T>(result: anyhow::Result<T>) -> String {
    format!("{:#}", result.err().expect("expected error"))
}
