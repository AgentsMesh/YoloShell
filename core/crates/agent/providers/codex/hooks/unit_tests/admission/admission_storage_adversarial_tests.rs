use std::os::unix::fs::PermissionsExt;

#[path = "../../test_support/admission.rs"]
mod support;
use support::*;

const CORRUPT_JSON: &[u8] = b"{";
const OUTSIDE_FILE_NAME: &str = "outside-admission.json";
const HARD_LINK_FILE_NAME: &str = "admission-hard-link.json";
const REPLACEMENT_FILE_NAME: &str = "admission-replacement.json";
const OUTSIDE_CONTENT: &[u8] = b"outside";
const PRIVATE_FILE_MODE: u32 = crate::paths::constants::PRIVATE_FILE_MODE;
const UNSAFE_FILE_MODE: u32 = 0o644;

#[test]
fn cold_read_rejects_unknown_fields_and_malformed_json() {
    let fixture = Fixture::new();
    let storage = fixture.storage();
    let mut unknown = serde_json::to_value(fixture.record()).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert(UNKNOWN_FIELD.into(), UNKNOWN_VALUE.into());
    write_raw(storage.path(), &serde_json::to_vec(&unknown).unwrap());
    assert!(storage.read().is_err());

    let fixture = Fixture::new();
    let storage = fixture.storage();
    write_raw(storage.path(), CORRUPT_JSON);
    assert!(storage.read().is_err());
}

#[test]
fn admission_path_never_follows_a_symbolic_link() {
    let fixture = Fixture::new();
    let storage = fixture.storage();
    crate::paths::directory::PrivateDirectory::ensure(fixture.artifacts.directory()).unwrap();
    let outside = fixture
        .request
        .hook_executable
        .parent()
        .unwrap()
        .join(OUTSIDE_FILE_NAME);
    std::fs::write(&outside, OUTSIDE_CONTENT).unwrap();
    std::os::unix::fs::symlink(&outside, storage.path()).unwrap();
    assert!(storage.publish(&fixture.record()).is_err());
    assert_eq!(std::fs::read(outside).unwrap(), OUTSIDE_CONTENT);
}

#[test]
fn cold_read_rejects_unsafe_permissions_and_hard_links() {
    let fixture = Fixture::new();
    let storage = fixture.storage();
    storage.publish(&fixture.record()).unwrap();
    std::fs::set_permissions(
        storage.path(),
        std::fs::Permissions::from_mode(UNSAFE_FILE_MODE),
    )
    .unwrap();
    assert!(storage.read().is_err());

    let fixture = Fixture::new();
    let storage = fixture.storage();
    storage.publish(&fixture.record()).unwrap();
    let hard_link = storage.path().with_file_name(HARD_LINK_FILE_NAME);
    std::fs::hard_link(storage.path(), hard_link).unwrap();
    assert!(storage.read().is_err());
}

#[test]
fn exact_file_identity_detects_same_payload_substitution() {
    let fixture = Fixture::new();
    let storage = fixture.storage();
    let published = storage.publish(&fixture.record()).unwrap().into_artifact();
    let bytes = std::fs::read(storage.path()).unwrap();
    let replacement = storage.path().with_file_name(REPLACEMENT_FILE_NAME);
    std::fs::write(&replacement, bytes).unwrap();
    std::fs::set_permissions(
        &replacement,
        std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
    )
    .unwrap();
    std::fs::rename(replacement, storage.path()).unwrap();
    let cold = storage.read().unwrap().unwrap();
    assert_eq!(cold.record(), published.record());
    assert_ne!(cold.file_identity(), published.file_identity());
    assert!(storage.verify(&published).is_err());
}

#[test]
fn cold_read_preserves_historical_authority_after_executables_disappear() {
    let fixture = Fixture::new();
    let storage = fixture.storage();
    let published = storage.publish(&fixture.record()).unwrap().into_artifact();
    for path in [
        &fixture.request.hook_executable,
        &fixture.request.real_executable,
    ] {
        std::fs::remove_file(path).unwrap();
    }
    let cold = storage.read().unwrap().unwrap();
    assert_eq!(cold, published);
    assert!(cold.record().verify_executables().is_err());
}

fn write_raw(path: &std::path::Path, contents: &[u8]) {
    crate::paths::private_file::PrivateFileEntry::ensure(path)
        .unwrap()
        .create_bytes_exclusive(contents, crate::paths::constants::MAX_DURABLE_JSON_BYTES)
        .unwrap();
}
