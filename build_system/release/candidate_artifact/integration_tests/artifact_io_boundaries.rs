use super::support::{ARCHIVE_BYTES, OTHER_ARCHIVE_BYTES};
use std::fs::{self, File};
use std::os::unix::fs::symlink;
use yolo_release_contracts::{ARCHIVE_NAME, ARTIFACT_CHANGED_ERROR, MAX_ARTIFACT_BYTES};
use yolo_release_filesystem::DirectoryHandle;

const COPY_NAME: &str = "copy.zip";
const SOURCE_NAME: &str = "source.zip";

#[test]
fn pinned_artifact_creation_never_overwrites_an_existing_or_aliased_name() {
    let root = tempfile::tempdir().unwrap();
    let handle = DirectoryHandle::open(root.path()).unwrap();
    assert!(crate::io::reopen(&handle).is_err());
    let target = root.path().join(SOURCE_NAME);
    fs::write(&target, OTHER_ARCHIVE_BYTES).unwrap();
    let archive = root.path().join(ARCHIVE_NAME);
    symlink(&target, &archive).unwrap();
    assert!(crate::io::create(&handle).is_err());
    assert!(crate::io::reopen(&handle).is_err());
    assert_eq!(fs::read(&target).unwrap(), OTHER_ARCHIVE_BYTES);
    fs::remove_file(&archive).unwrap();
    let file = crate::io::create(&handle).unwrap();
    assert!(crate::io::create(&handle).is_err());
    assert_eq!(file.metadata().unwrap().len(), 0);
}

#[test]
fn bounded_artifact_copy_rejects_size_overflow_and_source_truncation() {
    let root = tempfile::tempdir().unwrap();
    let source_path = root.path().join(SOURCE_NAME);
    fs::write(&source_path, ARCHIVE_BYTES).unwrap();
    let source = File::open(&source_path).unwrap();
    let mut destination = File::create(root.path().join(COPY_NAME)).unwrap();
    assert_eq!(
        crate::io::copy(&source, &mut destination, MAX_ARTIFACT_BYTES + 1)
            .unwrap_err()
            .to_string(),
        ARTIFACT_CHANGED_ERROR
    );
    File::create(&source_path).unwrap();
    assert_eq!(
        crate::io::copy(&source, &mut destination, ARCHIVE_BYTES.len() as u64)
            .unwrap_err()
            .to_string(),
        ARTIFACT_CHANGED_ERROR
    );
    assert_eq!(destination.metadata().unwrap().len(), 0);
}
