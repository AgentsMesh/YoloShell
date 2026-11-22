use super::{
    constants::*,
    fixture::{self, Fixture},
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use uuid::Uuid;
use yolo_distribution_bundle::{
    ArtifactRole, BIN_DIRECTORY, MANIFEST_FILE_NAME, MAX_ARTIFACT_BYTES,
};
use yolo_distribution_release_store::{CURRENT_FILE_NAME, MANIFEST_MODE, ReleaseStore};

#[test]
fn missing_extra_corrupt_linked_and_unsafe_stage_entries_fail_without_activation_or_cleanup() {
    for corruption in [
        Corruption::Missing,
        Corruption::Extra,
        Corruption::ExtraExecutable,
        Corruption::Digest,
        Corruption::Size,
        Corruption::Oversized,
        Corruption::Mode,
        Corruption::Symlink,
        Corruption::Hardlink,
        Corruption::Directory,
        Corruption::ManifestSymlink,
        Corruption::BinSymlink,
    ] {
        let fixture = Fixture::new();
        let store = fixture.store();
        let (stage, descriptor) = fixture.stage(FIRST_FILL);
        let path = store.stage_path(stage).unwrap();
        let artifact = path.join(ArtifactRole::Runtime.relative_path());
        let outside = fixture.root.join(OUTSIDE_NAME);
        match corruption {
            Corruption::Missing => fs::remove_file(&artifact).unwrap(),
            Corruption::Extra => fixture::write(&path.join(EXTRA_NAME), &[], MANIFEST_MODE),
            Corruption::ExtraExecutable => fixture::write(
                &path.join(BIN_DIRECTORY).join(EXTRA_NAME),
                &fs::read(&artifact).unwrap(),
                yolo_distribution_release_store::EXECUTABLE_MODE,
            ),
            Corruption::Digest => {
                let mut bytes = fs::read(&artifact).unwrap();
                *bytes.last_mut().unwrap() = SECOND_FILL;
                fs::write(&artifact, bytes).unwrap();
            }
            Corruption::Size => fs::write(&artifact, [FIRST_FILL]).unwrap(),
            Corruption::Oversized => fs::OpenOptions::new()
                .write(true)
                .open(&artifact)
                .unwrap()
                .set_len(MAX_ARTIFACT_BYTES + 1)
                .unwrap(),
            Corruption::Mode => {
                fs::set_permissions(&artifact, fs::Permissions::from_mode(UNSAFE_MODE)).unwrap()
            }
            Corruption::Symlink => {
                fs::rename(&artifact, &outside).unwrap();
                symlink(&outside, &artifact).unwrap();
            }
            Corruption::Hardlink => fs::hard_link(&artifact, &outside).unwrap(),
            Corruption::Directory => {
                fs::remove_file(&artifact).unwrap();
                fixture::directory(&artifact);
            }
            Corruption::ManifestSymlink => {
                let manifest = path.join(MANIFEST_FILE_NAME);
                fs::rename(&manifest, &outside).unwrap();
                symlink(&outside, &manifest).unwrap();
            }
            Corruption::BinSymlink => {
                let bin = path.join(BIN_DIRECTORY);
                fs::rename(&bin, &outside).unwrap();
                symlink(&outside, &bin).unwrap();
            }
        }
        assert!(
            store
                .publish(stage, &descriptor, None, Uuid::new_v4())
                .is_err()
        );
        assert!(path.exists());
        assert_eq!(store.inspect().unwrap(), None);
        assert_eq!(store.inspect_release(&descriptor).unwrap(), None);
    }
}

#[test]
fn valid_hashes_do_not_authorize_invalid_binary_headers_or_another_bundle_manifest() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let mut bytes = ArtifactRole::ALL.map(|role| fixture::payload(role, FIRST_FILL));
    bytes[0][0] = 0;
    let (stage, descriptor) = fixture.stage_payloads(bytes);
    assert!(
        store
            .publish(stage, &descriptor, None, Uuid::new_v4())
            .is_err()
    );
    let (stage, _) = fixture.stage(FIRST_FILL);
    let (_, other) = fixture.stage(SECOND_FILL);
    assert!(store.publish(stage, &other, None, Uuid::new_v4()).is_err());
    assert_eq!(store.inspect().unwrap(), None);
}

#[test]
fn namespace_replacement_is_not_adopted_even_when_original_paths_are_restored() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let displaced = fixture.temporary.path().join(DISPLACED_NAME);
    fs::rename(&fixture.root, &displaced).unwrap();
    fixture::directory(&fixture.root);
    assert!(store.inspect().is_err());
    let replacement = fixture.temporary.path().join(OUTSIDE_NAME);
    fs::rename(&fixture.root, &replacement).unwrap();
    fs::rename(&displaced, &fixture.root).unwrap();
    assert!(store.inspect().is_err());
    assert_eq!(
        ReleaseStore::open(&fixture.root)
            .unwrap()
            .inspect()
            .unwrap(),
        None
    );
}

#[test]
fn corrupt_or_linked_selector_does_not_grant_permission_to_reinitialize() {
    for linked in [false, true] {
        let fixture = Fixture::new();
        let store = fixture.store();
        let selector = fixture.root.join(CURRENT_FILE_NAME);
        if linked {
            let outside = fixture.root.join(OUTSIDE_NAME);
            fixture::write(&outside, &[], MANIFEST_MODE);
            symlink(&outside, &selector).unwrap();
        } else {
            fixture::write(&selector, &[], MANIFEST_MODE);
        }
        let (stage, descriptor) = fixture.stage(FIRST_FILL);
        assert!(store.inspect().is_err());
        assert!(
            store
                .publish(stage, &descriptor, None, Uuid::new_v4())
                .is_err()
        );
        assert!(store.stage_path(stage).unwrap().exists());
    }
    let fixture = Fixture::new();
    let store = fixture.store();
    let (stage, descriptor) = fixture.stage(FIRST_FILL);
    store
        .publish(stage, &descriptor, None, Uuid::new_v4())
        .unwrap();
    fs::write(
        descriptor.executable_path(&fixture.root, ArtifactRole::Runtime),
        [FIRST_FILL],
    )
    .unwrap();
    assert!(store.inspect().is_err());
    let (stage, replacement) = fixture.stage(SECOND_FILL);
    assert!(
        store
            .publish(stage, &replacement, None, Uuid::new_v4())
            .is_err()
    );
}

enum Corruption {
    Missing,
    Extra,
    ExtraExecutable,
    Digest,
    Size,
    Oversized,
    Mode,
    Symlink,
    Hardlink,
    Directory,
    ManifestSymlink,
    BinSymlink,
}
