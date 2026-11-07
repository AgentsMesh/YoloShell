use super::*;
use crate::paths::{atomic_write_json, identity::DurableFileVersion};

const NAMESPACE: &str = "durable version test";
const FILE_NAME: &str = "record.json";
const VALUE: u64 = 7;
const MALFORMED_BYTES: &[u8] = b"malformed json";

#[test]
fn namespace_json_returns_payload_and_both_versions_from_the_same_descriptor() {
    let root = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(&root.path().canonicalize().unwrap()).unwrap();
    let path = directory.path().join(FILE_NAME);
    let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)]).unwrap();
    namespace.write_json(&path, &VALUE).unwrap();
    let (value, live, durable): (u64, _, _) = namespace.read_json_with_versions(&path).unwrap();
    let opened = namespace
        .file(&path)
        .unwrap()
        .open_read(crate::paths::MAX_DURABLE_JSON_BYTES)
        .unwrap();
    assert_eq!(value, VALUE);
    assert_eq!(opened.identity(), live);
    assert_eq!(
        opened
            .durable_version(&path, crate::paths::MAX_DURABLE_JSON_BYTES)
            .unwrap(),
        durable
    );
    assert_eq!(
        durable
            .verify_file_bounded(opened.file(), &path, crate::paths::MAX_DURABLE_JSON_BYTES)
            .unwrap(),
        live
    );
}

#[test]
fn same_value_replacement_between_open_and_read_cannot_mix_version_authority() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FILE_NAME);
    atomic_write_json(&path, &VALUE).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();
    let result: Result<(
        u64,
        crate::paths::identity::FileIdentity,
        DurableFileVersion,
    )> = crate::paths::read_entry_guarded_with_versions(entry, || atomic_write_json(&path, &VALUE));
    assert!(result.is_err());
    assert_eq!(crate::paths::read_json::<u64>(&path).unwrap(), VALUE);
}

#[test]
fn versioned_namespace_reads_reject_missing_malformed_and_outside_anchor_files() {
    let root = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(&root.path().canonicalize().unwrap()).unwrap();
    let path = directory.path().join(FILE_NAME);
    let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)]).unwrap();
    assert!(namespace.read_json_with_versions::<u64>(&path).is_err());
    namespace.write_json(&path, &VALUE).unwrap();
    std::fs::write(&path, MALFORMED_BYTES).unwrap();
    assert!(namespace.read_json_with_versions::<u64>(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), MALFORMED_BYTES);
    let outside = tempfile::tempdir().unwrap();
    let foreign = outside.path().join(FILE_NAME);
    atomic_write_json(&foreign, &VALUE).unwrap();
    assert!(namespace.read_json_with_versions::<u64>(&foreign).is_err());
    assert_eq!(crate::paths::read_json::<u64>(&foreign).unwrap(), VALUE);
}
