use crate::file_seal::FileSeal;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, SystemTime};

const ORIGINAL_CONTENT: &[u8] = b"sealed";
const CHANGED_CONTENT: &[u8] = b"changed";
const CHANGED_ERROR: &str = "fixture changed";
const PRIVATE_MODE: u32 = 0o600;
const READ_ONLY_MODE: u32 = 0o400;
const FIXED_MODIFIED_TIME: Duration = Duration::from_secs(1_700_000_000);
const NANOS_CHANGE: Duration = Duration::from_nanos(1);
const SECONDS_CHANGE: Duration = Duration::from_secs(1);

#[test]
fn verifies_the_same_open_file_and_detects_mutation() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("evidence");
    fs::write(&path, ORIGINAL_CONTENT).unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let seal = FileSeal::capture(&file).unwrap();
    assert_eq!(seal.length(), ORIGINAL_CONTENT.len() as u64);
    seal.verify(&file, CHANGED_ERROR).unwrap();
    (&file).write_all(CHANGED_CONTENT).unwrap();
    assert!(seal.verify(&file, CHANGED_ERROR).is_err());
}

#[test]
fn preserves_identity_across_rename_and_rejects_directories() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let renamed = root.path().join("renamed");
    fs::write(&source, ORIGINAL_CONTENT).unwrap();
    let file = File::open(&source).unwrap();
    let seal = FileSeal::capture(&file).unwrap();
    fs::rename(source, renamed).unwrap();
    seal.recapture_after_rename(&file, CHANGED_ERROR).unwrap();
    assert!(FileSeal::capture(&File::open(root.path()).unwrap()).is_err());
}

#[test]
fn rename_recapture_rejects_a_different_inode_with_identical_contents() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let replacement = root.path().join("replacement");
    fs::write(&source, ORIGINAL_CONTENT).unwrap();
    fs::write(&replacement, ORIGINAL_CONTENT).unwrap();
    let seal = FileSeal::capture(&File::open(source).unwrap()).unwrap();
    assert_eq!(
        seal.recapture_after_rename(&File::open(replacement).unwrap(), CHANGED_ERROR)
            .unwrap_err()
            .to_string(),
        CHANGED_ERROR
    );
}

#[test]
fn rename_recapture_rejects_length_mode_link_and_timestamp_changes() {
    enum Mutation {
        Length,
        Mode,
        Link,
        ModifiedSeconds,
        ModifiedNanoseconds,
    }
    for mutation in [
        Mutation::Length,
        Mutation::Mode,
        Mutation::Link,
        Mutation::ModifiedSeconds,
        Mutation::ModifiedNanoseconds,
    ] {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::write(&source, ORIGINAL_CONTENT).unwrap();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&source)
            .unwrap();
        file.set_permissions(fs::Permissions::from_mode(PRIVATE_MODE))
            .unwrap();
        let modified = SystemTime::UNIX_EPOCH + FIXED_MODIFIED_TIME;
        file.set_modified(modified).unwrap();
        let seal = FileSeal::capture(&file).unwrap();
        match mutation {
            Mutation::Length => file.set_len(CHANGED_CONTENT.len() as u64).unwrap(),
            Mutation::Mode => file
                .set_permissions(fs::Permissions::from_mode(READ_ONLY_MODE))
                .unwrap(),
            Mutation::Link => fs::hard_link(&source, root.path().join("link")).unwrap(),
            Mutation::ModifiedSeconds => file.set_modified(modified + SECONDS_CHANGE).unwrap(),
            Mutation::ModifiedNanoseconds => file.set_modified(modified + NANOS_CHANGE).unwrap(),
        }
        assert_eq!(
            seal.recapture_after_rename(&file, CHANGED_ERROR)
                .unwrap_err()
                .to_string(),
            CHANGED_ERROR
        );
    }
}
