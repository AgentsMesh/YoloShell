use super::*;
use crate::paths::constants::PRIVATE_FILE_MODE;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

const TARGET_FILE_NAME: &str = "retained.json";
const REPLACEMENT_FILE_NAME: &str = "replacement.json";
const PRIVATE_DIRECTORY_NAME: &str = "private";
const JSON_VALUE: &str = "retained-observation";
const UNLINKED_FILE_LINK_COUNT: u64 = 0;

#[test]
fn observation_retains_the_exact_descriptor_across_same_value_replacement() {
    let root = tempfile::tempdir().unwrap();
    let directory = crate::paths::directory::PrivateDirectory::ensure(
        &root.path().join(PRIVATE_DIRECTORY_NAME),
    )
    .unwrap();
    let target = directory.path().join(TARGET_FILE_NAME);
    let replacement = directory.path().join(REPLACEMENT_FILE_NAME);
    write_json(&target, JSON_VALUE);
    let (value, observation) = RetainedJsonFileObservation::bind::<String>(&target).unwrap();
    assert_eq!(value, JSON_VALUE);

    write_json(&replacement, JSON_VALUE);
    std::fs::rename(&replacement, &target).unwrap();

    assert_eq!(
        observation._file.metadata().unwrap().nlink(),
        UNLINKED_FILE_LINK_COUNT
    );
    assert!(observation.verify_unchanged().is_err());
}

fn write_json(path: &Path, value: &str) {
    let encoded = serde_json::to_vec(value).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(&encoded).unwrap();
    file.sync_all().unwrap();
}
