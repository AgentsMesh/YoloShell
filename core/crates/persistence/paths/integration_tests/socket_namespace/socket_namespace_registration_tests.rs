use super::*;
use crate::paths::directory::PrivateDirectory;
use std::fs;

const ORIGINAL_HOME: &str = "home";
const OTHER_HOME: &str = "other";
const SAVED_REGISTRATION: &str = "saved-registration";
const CORRUPT_RECORD: &[u8] = b"untrusted registration";
const SAVED_HOME: &str = "saved-home";

fn expected() -> NamespaceBinding {
    NamespaceBinding {
        schema_version: super::super::SOCKET_BINDING_SCHEMA_VERSION,
        effective_user_id: unsafe { libc::geteuid() },
        home_digest: "a".repeat(super::super::SOCKET_FULL_HASH_HEX_LENGTH),
    }
}

#[test]
fn inspect_missing_is_read_only_and_shared_registration_blocks_exclusive_recovery() {
    let root = tempfile::tempdir().unwrap();
    let home = PrivateDirectory::ensure(&root.path().join(ORIGINAL_HOME)).unwrap();
    assert!(inspect(home.path(), &expected()).unwrap().is_none());
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
    assert!(retain(home.path(), &expected(), false).is_err());
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
    let first = retain(home.path(), &expected(), true).unwrap();
    let second = retain(home.path(), &expected(), false).unwrap();
    assert_eq!(first.identity(), second.identity());
    assert_eq!(
        inspect(home.path(), &expected()).unwrap(),
        Some(first.durable_identity())
    );
    assert!(
        Registration::try_existing(home.path(), &expected(), Mode::Exclusive)
            .unwrap()
            .is_none()
    );
    drop(first);
    assert!(
        Registration::try_existing(home.path(), &expected(), Mode::Exclusive)
            .unwrap()
            .is_none()
    );
    drop(second);
    let exclusive = Registration::try_existing(home.path(), &expected(), Mode::Exclusive)
        .unwrap()
        .unwrap();
    assert!(retain(home.path(), &expected(), true).is_err());
    exclusive.verify().unwrap();
    drop(exclusive);
    retain(home.path(), &expected(), false)
        .unwrap()
        .verify()
        .unwrap();
}

#[test]
fn invalid_conflicting_or_copied_registrations_are_never_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let home = PrivateDirectory::ensure(&root.path().join(ORIGINAL_HOME)).unwrap();
    let registration = retain(home.path(), &expected(), true).unwrap();
    let path = home.path().join(REGISTRATION_FILE);
    let bytes = fs::read(&path).unwrap();
    let mut conflict = expected();
    conflict.home_digest = "b".repeat(conflict.home_digest.len());
    assert!(retain(home.path(), &conflict, true).is_err());
    assert!(inspect(home.path(), &conflict).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let other = PrivateDirectory::ensure(&root.path().join(OTHER_HOME)).unwrap();
    fs::copy(&path, other.path().join(REGISTRATION_FILE)).unwrap();
    assert!(inspect(other.path(), &expected()).is_err());
    assert!(retain(other.path(), &expected(), true).is_err());
    fs::write(&path, CORRUPT_RECORD).unwrap();
    assert!(registration.verify().is_err());
    assert!(retain(home.path(), &expected(), true).is_err());
    assert_eq!(fs::read(&path).unwrap(), CORRUPT_RECORD);
}

#[test]
fn a_replaced_registration_has_a_new_identity_and_cannot_refresh_an_old_lease() {
    let root = tempfile::tempdir().unwrap();
    let home = PrivateDirectory::ensure(&root.path().join(ORIGINAL_HOME)).unwrap();
    let original = retain(home.path(), &expected(), true).unwrap();
    let path = home.path().join(REGISTRATION_FILE);
    fs::rename(&path, root.path().join(SAVED_REGISTRATION)).unwrap();
    let replacement = retain(home.path(), &expected(), true).unwrap();
    assert_ne!(original.identity(), replacement.identity());
    assert!(original.verify().is_err());
    assert_eq!(
        inspect(home.path(), &expected()).unwrap(),
        Some(replacement.durable_identity())
    );
    replacement.verify().unwrap();
}

#[test]
fn home_disappearing_after_record_read_is_not_reported_as_an_absent_registration() {
    for acquire in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(ORIGINAL_HOME);
        let home = PrivateDirectory::ensure(&path).unwrap();
        drop(retain(home.path(), &expected(), true).unwrap());
        let saved = root.path().join(SAVED_HOME);
        let source = path.clone();
        read::test_support::install(move || fs::rename(source, saved).unwrap());
        if acquire {
            assert!(Registration::try_existing(&path, &expected(), Mode::Shared).is_err());
        } else {
            assert!(inspect(&path, &expected()).is_err());
        }
        assert!(!path.exists());
    }
}

#[test]
fn upgrade_waits_for_other_shared_owners_and_never_adopts_a_replacement() {
    let root = tempfile::tempdir().unwrap();
    let home = PrivateDirectory::ensure(&root.path().join(ORIGINAL_HOME)).unwrap();
    let mut first = retain(home.path(), &expected(), true).unwrap();
    let peer = retain(home.path(), &expected(), false).unwrap();
    assert!(!first.try_upgrade().unwrap());
    assert!(first.verify().is_err());
    peer.verify().unwrap();
    drop(peer);
    assert!(first.try_upgrade().unwrap());
    first.verify().unwrap();
    fs::rename(
        home.path().join(REGISTRATION_FILE),
        root.path().join(SAVED_REGISTRATION),
    )
    .unwrap();
    let replacement = retain(home.path(), &expected(), true).unwrap();
    assert!(first.try_upgrade().is_err());
    replacement.verify().unwrap();
}
