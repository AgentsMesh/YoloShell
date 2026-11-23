use super::support::{ARCHIVE_BYTES, OTHER_ARCHIVE_BYTES, error_text, fifo};
use crate::PinnedArtifact;
use std::fs::{self, File};
use std::io::Read;
use yolo_release_contracts::{ARCHIVE_NAME, MAX_ARTIFACT_BYTES};
use yolo_release_foundation::digest::bytes_sha256;

#[test]
fn pinned_artifact_survives_source_path_replacement() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join(ARCHIVE_NAME);
    fs::write(&source, ARCHIVE_BYTES).unwrap();
    let artifact = PinnedArtifact::open(&source).unwrap();
    fs::rename(&source, root.path().join("original.zip")).unwrap();
    fs::write(&source, OTHER_ARCHIVE_BYTES).unwrap();
    assert_eq!(artifact.sha256().unwrap(), bytes_sha256(ARCHIVE_BYTES));
    assert_eq!(artifact.size_bytes().unwrap(), ARCHIVE_BYTES.len() as u64);
}

#[test]
fn pinned_artifact_copies_exact_descriptor_bytes() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    let destination_path = root.path().join("copy.zip");
    let mut destination = File::create(&destination_path).unwrap();
    artifact.copy_to(&mut destination).unwrap();
    drop(destination);
    let mut bytes = Vec::new();
    File::open(destination_path)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, ARCHIVE_BYTES);
}

#[test]
fn artifact_name_and_type_are_fixed() {
    let root = tempfile::tempdir().unwrap();
    let wrong = root.path().join("Other.zip");
    fs::write(&wrong, ARCHIVE_BYTES).unwrap();
    assert!(error_text(PinnedArtifact::open(&wrong)).contains("unexpected name"));
    let directory = root.path().join(ARCHIVE_NAME);
    fs::create_dir(&directory).unwrap();
    assert!(error_text(PinnedArtifact::open(&directory)).contains("must be a file"));
}

#[test]
fn artifact_fifo_is_rejected_without_blocking() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ARCHIVE_NAME);
    fifo(&path);
    assert!(error_text(PinnedArtifact::open(&path)).contains("must be a file"));
}

#[test]
fn oversized_sparse_artifact_is_rejected_before_reading() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ARCHIVE_NAME);
    File::create(&path)
        .unwrap()
        .set_len(MAX_ARTIFACT_BYTES + 1)
        .unwrap();
    assert!(error_text(PinnedArtifact::open(&path)).contains("bounded limit"));
}
