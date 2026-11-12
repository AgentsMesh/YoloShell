#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use yolo_distribution_bundle::{ArtifactRole, BIN_DIRECTORY, BundleSource, MANIFEST_FILE_NAME};
use yolo_distribution_bundle_source::VerifiedBundle;

fn fixture() -> (tempfile::TempDir, BundleSource) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join(SOURCE);
    fs::create_dir_all(root.join(BIN_DIRECTORY)).unwrap();
    let stage = yolo_remote_bundle_test_support::fixtures::stage();
    fs::write(
        root.join(MANIFEST_FILE_NAME),
        serde_json::to_vec(stage.descriptor()).unwrap(),
    )
    .unwrap();
    fs::set_permissions(
        root.join(MANIFEST_FILE_NAME),
        fs::Permissions::from_mode(MANIFEST_MODE),
    )
    .unwrap();
    for role in ArtifactRole::ALL {
        let path = root.join(role.relative_path());
        fs::write(&path, stage.payload(role)).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(MODE)).unwrap();
    }
    (
        temp,
        BundleSource::new(root, stage.descriptor().clone()).unwrap(),
    )
}

#[test]
fn exact_manifest_roles_and_bounded_bytes_remain_bound_to_retained_files() {
    let (_temp, source) = fixture();
    let verified = VerifiedBundle::open(&source).unwrap();
    assert_eq!(verified.source(), &source);
    for role in ArtifactRole::ALL {
        source
            .descriptor()
            .verify_artifact(role, &verified.payload(role).unwrap())
            .unwrap();
    }
    verified.verify().unwrap();
}

#[test]
fn same_bytes_file_or_root_substitution_invalidates_retained_bundle() {
    for root in [false, true] {
        let (temp, source) = fixture();
        let verified = VerifiedBundle::open(&source).unwrap();
        let saved = temp.path().join(SAVED);
        if root {
            fs::rename(source.root(), &saved).unwrap();
            fs::create_dir(source.root()).unwrap();
        } else {
            let file = source.root().join(ArtifactRole::Runtime.relative_path());
            fs::rename(&file, &saved).unwrap();
            fs::copy(saved, &file).unwrap();
        }
        assert!(verified.verify().is_err());
        assert!(verified.payload(ArtifactRole::Runtime).is_err());
    }
}

#[test]
fn symlink_unsafe_permissions_invalid_bytes_and_wrong_manifest_are_rejected() {
    for corruption in [
        Corruption::Symlink,
        Corruption::Mode,
        Corruption::Bytes,
        Corruption::Manifest,
    ] {
        let (temp, source) = fixture();
        let path = source.root().join(ArtifactRole::Runtime.relative_path());
        match corruption {
            Corruption::Symlink => {
                let saved = temp.path().join(SAVED);
                fs::rename(&path, &saved).unwrap();
                symlink(saved, path).unwrap();
            }
            Corruption::Mode => {
                fs::set_permissions(path, fs::Permissions::from_mode(UNSAFE_MODE)).unwrap()
            }
            Corruption::Bytes => fs::write(path, INVALID_BYTES).unwrap(),
            Corruption::Manifest => {
                fs::write(source.root().join(MANIFEST_FILE_NAME), INVALID_BYTES).unwrap()
            }
        }
        assert!(VerifiedBundle::open(&source).is_err());
    }
}

enum Corruption {
    Symlink,
    Mode,
    Bytes,
    Manifest,
}

#[test]
fn oversized_sparse_image_is_rejected_before_allocating_or_reading_payloads() {
    let (_temp, source) = fixture();
    let path = source.root().join(ArtifactRole::Runtime.relative_path());
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_len(yolo_distribution_bundle::MAX_ARTIFACT_BYTES + 1)
        .unwrap();
    assert!(VerifiedBundle::open(&source).is_err());
}
