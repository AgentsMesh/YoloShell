use super::*;
use std::os::unix::fs::PermissionsExt;
#[path = "../test_support/runtime_artifact_bounds_constants.rs"]
mod constants;
use constants::*;

#[test]
fn runtime_reader_accepts_the_unified_image_and_rejects_the_shared_limit_overflow() {
    assert_eq!(
        MAX_ARTIFACT_FILE_BYTES,
        yolo_runtime::constants::MAX_RUNTIME_BINARY_BYTES
    );
    let artifact = tempfile::NamedTempFile::new().unwrap();
    artifact
        .as_file()
        .set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    for length in [
        RETIRED_ARTIFACT_LIMIT + SIZE_BOUNDARY_STEP,
        UNIFIED_COVERAGE_IMAGE_BYTES,
    ] {
        artifact.as_file().set_len(length).unwrap();
        let bytes = read_trusted_file(artifact.path(), ARTIFACT_LABEL).unwrap();
        assert_eq!(bytes.len() as u64, length);
    }
    artifact
        .as_file()
        .set_len(MAX_ARTIFACT_FILE_BYTES as u64)
        .unwrap();
    let metadata = inspect_secure_file(
        artifact.as_file(),
        artifact.path(),
        ARTIFACT_LABEL,
        READ_ONLY_FILE_MODE_MASK,
        true,
        MAX_ARTIFACT_FILE_BYTES,
    )
    .unwrap();
    assert_eq!(metadata.len(), MAX_ARTIFACT_FILE_BYTES as u64);
    artifact
        .as_file()
        .set_len(MAX_ARTIFACT_FILE_BYTES as u64 + SIZE_BOUNDARY_STEP)
        .unwrap();
    assert!(read_trusted_file(artifact.path(), ARTIFACT_LABEL).is_err());
}

#[test]
fn the_runtime_exception_does_not_expand_config_known_hosts_or_private_key_reads() {
    let artifact = tempfile::NamedTempFile::new().unwrap();
    artifact
        .as_file()
        .set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .unwrap();
    artifact
        .as_file()
        .set_len(RETIRED_ARTIFACT_LIMIT + SIZE_BOUNDARY_STEP)
        .unwrap();
    assert!(read_trusted_file(artifact.path(), KNOWN_HOSTS_LABEL).is_err());
    assert!(read_trusted_file(artifact.path(), CONFIG_LABEL).is_err());
    assert!(read_private_file(artifact.path(), CONFIG_LABEL).is_err());
    assert!(read_identity(artifact.path()).is_err());
}
