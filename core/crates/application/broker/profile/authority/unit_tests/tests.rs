use super::test_fixture::*;
use super::*;
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{PermissionsExt, symlink};

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn start_requires_exact_committed_host_and_digest() {
    let fixture = Fixture::new(committed());
    assert_eq!(
        digest::profile_sha256(&full_profile()).unwrap(),
        PROFILE_SHA256
    );
    let guard =
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).unwrap();
    guard.verify_config(HOST_ID, PROFILE_SHA256).unwrap();
    assert!(guard.verify_config(OTHER_HOST_ID, PROFILE_SHA256).is_err());
    assert!(guard.verify_config(HOST_ID, OTHER_PROFILE_SHA256).is_err());
    drop(guard);
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, OTHER_PROFILE_SHA256)
            .is_err()
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn retirement_requires_exact_blocked_mutation() {
    let mutation_id = Uuid::new_v4();
    let fixture = Fixture::new(blocked(mutation_id));
    RetirementAuthorityGuard::acquire(&fixture.document, &fixture.lock, HOST_ID, mutation_id)
        .unwrap();
    assert!(
        RetirementAuthorityGuard::acquire(
            &fixture.document,
            &fixture.lock,
            HOST_ID,
            Uuid::new_v4()
        )
        .is_err()
    );
    assert!(
        RetirementAuthorityGuard::acquire(
            &fixture.document,
            &fixture.lock,
            OTHER_HOST_ID,
            mutation_id
        )
        .is_err()
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn authority_files_reject_symlinks_links_and_unsafe_modes() {
    let fixture = Fixture::new(committed());
    let saved_lock = fixture.lock.with_extension("saved");
    fs::rename(&fixture.lock, &saved_lock).unwrap();
    symlink(&saved_lock, &fixture.lock).unwrap();
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).is_err()
    );

    let fixture = Fixture::new(committed());
    fs::set_permissions(&fixture.document, fs::Permissions::from_mode(UNSAFE_MODE)).unwrap();
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).is_err()
    );

    let fixture = Fixture::new(committed());
    fs::hard_link(&fixture.document, fixture.document.with_extension("hard")).unwrap();
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).is_err()
    );
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn guard_holds_lock_and_detects_named_replacement() {
    let fixture = Fixture::new(committed());
    let guard =
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).unwrap();
    let contender = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&fixture.lock)
        .unwrap();
    let result = unsafe { libc::flock(contender.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    assert_ne!(result, 0);

    let displaced = fixture.lock.with_extension("displaced");
    fs::rename(&fixture.lock, displaced).unwrap();
    write_private(&fixture.lock, LOCK_MARKER);
    assert!(guard.verify().is_err());
}

#[cfg(feature = "yolo-unit-tests")]
#[path = "document_tests.rs"]
mod document_tests;
