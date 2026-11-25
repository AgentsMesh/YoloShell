use super::support::{ARCHIVE_BYTES, error_text};
use crate::{ARTIFACT_CHANGED_ERROR, PinnedArtifact, UNSEALED_ERROR};
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::PRIVATE_FILE_MODE;

const SIZE_INCREMENT: u64 = 1;
const CORRUPTED_HASH: &str = "corrupted";
const COPY_NAME: &str = "copy.zip";

#[test]
fn every_artifact_accessor_requires_a_sealed_custody_root() {
    let root = tempfile::tempdir().unwrap();
    let mut artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    artifact.root_seal = None;
    assert_eq!(error_text(artifact.sha256()), UNSEALED_ERROR);
    assert_eq!(error_text(artifact.size_bytes()), UNSEALED_ERROR);
    assert_eq!(error_text(artifact.independent_reader()), UNSEALED_ERROR);
    let mut destination = File::create(root.path().join(COPY_NAME)).unwrap();
    assert_eq!(
        error_text(artifact.copy_to(&mut destination)),
        UNSEALED_ERROR
    );
    assert_eq!(destination.metadata().unwrap().len(), u64::default());
}

#[test]
fn cached_artifact_digest_and_size_cannot_override_retained_file_evidence() {
    let root = tempfile::tempdir().unwrap();
    let mut artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    artifact.size_bytes += SIZE_INCREMENT;
    assert_eq!(error_text(artifact.size_bytes()), ARTIFACT_CHANGED_ERROR);
    artifact.sha256 = CORRUPTED_HASH.to_owned();
    assert_eq!(error_text(artifact.sha256()), ARTIFACT_CHANGED_ERROR);
}

#[test]
fn retained_artifact_permission_changes_are_detected_before_copying() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    artifact
        .file
        .set_permissions(std::fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    let mut destination = File::create(root.path().join(COPY_NAME)).unwrap();
    assert!(artifact.copy_to(&mut destination).is_err());
    assert_eq!(destination.metadata().unwrap().len(), u64::default());
}

#[test]
fn read_only_copy_destination_is_rejected_without_modifying_the_pinned_source() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    let digest = artifact.sha256().unwrap();
    let mut destination = File::open(root.path().join(crate::ARCHIVE_NAME)).unwrap();
    assert!(artifact.copy_to(&mut destination).is_err());
    assert_eq!(artifact.sha256().unwrap(), digest);
}
