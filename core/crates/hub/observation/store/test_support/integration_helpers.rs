use crate::{constants::*, fixture::*};
use serde_json::Value;
use std::{fs, path::Path};
use yolo_paths::{
    constants::MAX_DURABLE_JSON_BYTES, directory::PrivateDirectory, identity::FileIdentity,
};

pub fn committed() -> Fixture {
    let fixture = Fixture::new();
    fixture
        .open()
        .unwrap()
        .record_history(&fixture.history())
        .unwrap();
    fixture
}

pub fn document(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

pub fn file_state(path: &Path) -> (Vec<u8>, FileIdentity) {
    let bytes = fs::read(path).unwrap();
    let identity =
        FileIdentity::inspect_bounded(&fs::metadata(path).unwrap(), path, MAX_DURABLE_JSON_BYTES)
            .unwrap();
    (bytes, identity)
}

pub fn reject_unchanged(fixture: &Fixture, path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    let before = fs::metadata(path).unwrap();
    assert!(fixture.open().is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
    let after = fs::metadata(path).unwrap();
    use std::os::unix::fs::MetadataExt;
    assert_eq!(
        (
            before.ino(),
            before.len(),
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec()
        ),
        (
            after.ino(),
            after.len(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec()
        )
    );
}

pub fn clone_records(fixture: &Fixture, destination: &Path) {
    PrivateDirectory::open_or_create_strict(destination).unwrap();
    for entry in fs::read_dir(&fixture.path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != LOCK_FILE {
            write_new(
                &destination.join(entry.file_name()),
                &fs::read(entry.path()).unwrap(),
            );
        }
    }
}
