use crate::{DurableObjectIdentity, ObjectKind};
use std::fs::File;
use std::io::Write;
use std::os::fd::AsFd;

const TEST_TMPDIR: &str = "TEST_TMPDIR";
const ORIGINAL_NAME: &str = "original";
const RENAMED_NAME: &str = "renamed";
const COPIED_NAME: &str = "copied";
const FIRST_CONTENT: &[u8] = b"first";
const SECOND_CONTENT: &[u8] = b"different content and length";

fn directory() -> tempfile::TempDir {
    let parent = std::env::var_os(TEST_TMPDIR)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    tempfile::tempdir_in(parent).unwrap()
}

#[test]
fn reopening_same_directory_preserves_durable_proof() {
    let directory = directory();
    let first = File::open(directory.path()).unwrap();
    let identity = DurableObjectIdentity::capture(first.as_fd()).unwrap();
    assert_eq!(identity.kind(), ObjectKind::Directory);
    drop(first);
    let reopened = File::open(directory.path()).unwrap();
    identity.verify(reopened.as_fd()).unwrap();
    let encoded = serde_json::to_vec(&identity).unwrap();
    serde_json::from_slice::<DurableObjectIdentity>(&encoded)
        .unwrap()
        .verify(reopened.as_fd())
        .unwrap();
}

#[test]
fn rename_preserves_object_but_replacement_and_copy_do_not() {
    let directory = directory();
    let original = directory.path().join(ORIGINAL_NAME);
    let renamed = directory.path().join(RENAMED_NAME);
    std::fs::create_dir(&original).unwrap();
    let retained = File::open(&original).unwrap();
    let identity = DurableObjectIdentity::capture(retained.as_fd()).unwrap();
    std::fs::rename(&original, &renamed).unwrap();
    identity
        .verify(File::open(&renamed).unwrap().as_fd())
        .unwrap();
    std::fs::create_dir(&original).unwrap();
    assert!(
        identity
            .verify(File::open(&original).unwrap().as_fd())
            .is_err()
    );
    identity.verify(retained.as_fd()).unwrap();
}

#[test]
fn content_changes_do_not_change_object_identity_but_same_value_copy_does() {
    let directory = directory();
    let original = directory.path().join(ORIGINAL_NAME);
    let copy = directory.path().join(COPIED_NAME);
    let mut file = File::create(&original).unwrap();
    file.write_all(FIRST_CONTENT).unwrap();
    let identity = DurableObjectIdentity::capture(file.as_fd()).unwrap();
    assert_eq!(identity.kind(), ObjectKind::RegularFile);
    file.write_all(SECOND_CONTENT).unwrap();
    identity.verify(file.as_fd()).unwrap();
    std::fs::copy(&original, &copy).unwrap();
    assert!(identity.verify(File::open(&copy).unwrap().as_fd()).is_err());
}

#[test]
fn socket_descriptor_cannot_become_durable_file_authority() {
    let (socket, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    assert!(DurableObjectIdentity::capture(socket.as_fd()).is_err());
}
