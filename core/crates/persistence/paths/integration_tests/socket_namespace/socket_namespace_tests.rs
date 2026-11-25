use super::{test_support::*, *};
use std::{ffi::CString, fs, os::fd::RawFd, os::unix::ffi::OsStrExt};

const INVALID_DIRECTORY_DESCRIPTOR: RawFd = -1;
const MISSING_CLEANUP_ENTRY: &str = "missing-cleanup-entry";
const LONG_COMPONENT: &str = "rules-apple-integration-scratch-component";
const LONG_COMPONENT_COUNT: usize = 8;
const REPLACEMENT_FILE: &str = "replacement";
const REPLACEMENT_BYTES: &[u8] = b"preserve replacement";
const SAVED_DIRECTORY: &str = "saved";
const DIGEST_BYTE_INCREMENT: u8 = 1;

#[test]
fn fresh_namespace_has_a_certificate_and_no_regular_ownership_files() {
    let fixture = RegisteredFixture::new();
    let owner = fixture.paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    assert!(cleanup::is_empty(directory.descriptor()).unwrap());
    assert!(directory_certificate::read(&directory).unwrap().is_some());
    owner.verify().unwrap();
    assert_eq!(owner.release().unwrap(), NamespaceRelease::Removed);
    assert!(!directory.path().exists());
}

#[test]
fn cleanup_io_reports_invalid_directories_and_missing_entries() {
    assert!(cleanup::is_empty(INVALID_DIRECTORY_DESCRIPTOR).is_err());
    let temporary = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(temporary.path()).unwrap();
    let missing = CString::new(MISSING_CLEANUP_ENTRY).unwrap();
    assert!(
        cleanup::unlink(
            directory.descriptor(),
            &missing,
            SOCKET_NAMESPACE_FILE_UNLINK_FLAGS
        )
        .is_err()
    );
}

#[test]
fn long_homes_produce_bounded_host_and_worker_endpoints() {
    let root = tempfile::tempdir().unwrap();
    let mut home = root.path().to_path_buf();
    for _ in 0..LONG_COMPONENT_COUNT {
        home.push(LONG_COMPONENT);
    }
    let paths = Paths::new(&home);
    for endpoint in [
        paths.host_socket().unwrap(),
        paths.hub_socket().unwrap(),
        paths.worker_socket(Uuid::new_v4()).unwrap(),
        paths.custody_guardian_socket(Uuid::new_v4()).unwrap(),
    ] {
        assert!(!endpoint.starts_with(&home));
        assert!(endpoint.as_os_str().as_bytes().len() < UNIX_SOCKET_PATH_MAX_BYTES);
    }
}

#[test]
fn full_certificate_rejects_a_forced_short_hash_collision() {
    let fixture = RegisteredFixture::new();
    let owner = fixture.paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let original = directory_certificate::read(&directory).unwrap().unwrap();
    let mut conflicting = owner.location.clone();
    let mut digest = conflicting.binding.home_digest.into_bytes();
    let index = SOCKET_HASH_HEX_LENGTH;
    digest[index] = if digest[index] == b'0' {
        b'0' + DIGEST_BYTE_INCREMENT
    } else {
        b'0'
    };
    conflicting.binding.home_digest = String::from_utf8(digest).unwrap();
    assert!(admission::retain(fixture.paths.home(), conflicting).is_err());
    assert_eq!(
        directory_certificate::read(&directory).unwrap(),
        Some(original)
    );
    owner.verify().unwrap();
}

#[test]
fn cleanup_preserves_replacement_directories_and_owned_data() {
    let fixture = RegisteredFixture::new();
    let owner = fixture.paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let saved = owner.root.path().join(SAVED_DIRECTORY);
    fs::rename(directory.path(), &saved).unwrap();
    let replacement = PrivateDirectory::ensure(directory.path()).unwrap();
    fs::write(replacement.path().join(REPLACEMENT_FILE), REPLACEMENT_BYTES).unwrap();
    assert!(owner.release().is_err());
    assert_eq!(
        fs::read(replacement.path().join(REPLACEMENT_FILE)).unwrap(),
        REPLACEMENT_BYTES
    );
    assert!(saved.exists());
}

#[cfg(target_os = "macos")]
#[test]
fn system_path_aliases_share_one_socket_namespace() {
    let suffix = format!("tmp/yolo-alias-{}", Uuid::new_v4());
    let visible = Paths::new(Path::new("/var").join(&suffix));
    let physical = Paths::new(Path::new("/private/var").join(&suffix));
    assert_eq!(visible.home(), physical.home());
    assert_eq!(
        visible.socket_dir().unwrap(),
        physical.socket_dir().unwrap()
    );
}
