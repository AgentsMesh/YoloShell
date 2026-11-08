use super::*;
use crate::remote_broker::constants::CONFIG_LABEL;
use std::fs::{self, File};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const ROOT_OWNED_FILE: &str = "/etc/hosts";
const ORIGINAL_FILE: &str = "original";
const REPLACEMENT_FILE: &str = "replacement";
const PRIVATE_MODE: u32 = 0o600;
const UNSAFE_MODE_BITS: u32 = 0o022;
const FILE_BYTES: &[u8] = b"secure";
const LONGER_FILE_BYTES: &[u8] = b"secure-longer";

#[test]
fn unix_metadata_distinguishes_root_policy_mode_and_link_count() {
    let root_metadata = File::open(ROOT_OWNED_FILE).unwrap().metadata().unwrap();
    assert_eq!(root_metadata.uid(), 0);
    assert_ne!(effective_uid(), 0);
    assert!(
        inspect_unix_metadata(
            &root_metadata,
            Path::new(ROOT_OWNED_FILE),
            CONFIG_LABEL,
            0,
            false,
        )
        .is_err()
    );

    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ORIGINAL_FILE);
    write_private(&path, FILE_BYTES);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(PRIVATE_MODE | UNSAFE_MODE_BITS);
    fs::set_permissions(&path, permissions).unwrap();
    assert!(
        inspect_unix_metadata(
            &fs::metadata(&path).unwrap(),
            &path,
            CONFIG_LABEL,
            UNSAFE_MODE_BITS,
            false,
        )
        .is_err()
    );

    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
    fs::hard_link(&path, root.path().join(REPLACEMENT_FILE)).unwrap();
    assert!(
        inspect_unix_metadata(
            &fs::metadata(&path).unwrap(),
            &path,
            CONFIG_LABEL,
            UNSAFE_MODE_BITS,
            false,
        )
        .is_err()
    );
}

#[test]
fn stable_file_verification_rejects_inode_length_read_and_path_changes() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join(ORIGINAL_FILE);
    let replacement = root.path().join(REPLACEMENT_FILE);
    write_private(&original, FILE_BYTES);
    write_private(&replacement, FILE_BYTES);
    let before = fs::metadata(&original).unwrap();
    let replacement_metadata = fs::metadata(&replacement).unwrap();
    assert!(
        verify_secure_file_stable(
            &original,
            CONFIG_LABEL,
            &before,
            &replacement_metadata,
            FILE_BYTES.len(),
        )
        .is_err()
    );

    fs::write(&original, LONGER_FILE_BYTES).unwrap();
    let longer = fs::metadata(&original).unwrap();
    assert!(
        verify_secure_file_stable(
            &original,
            CONFIG_LABEL,
            &before,
            &longer,
            LONGER_FILE_BYTES.len(),
        )
        .is_err()
    );

    write_private(&original, FILE_BYTES);
    let stable = fs::metadata(&original).unwrap();
    assert!(
        verify_secure_file_stable(
            &original,
            CONFIG_LABEL,
            &stable,
            &stable,
            FILE_BYTES.len() + 1,
        )
        .is_err()
    );

    fs::rename(&replacement, &original).unwrap();
    assert!(
        verify_secure_file_stable(&original, CONFIG_LABEL, &stable, &stable, FILE_BYTES.len(),)
            .is_err()
    );
}

fn write_private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
}
