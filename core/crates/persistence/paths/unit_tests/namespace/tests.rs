use super::*;

const MANAGED_NAME: &str = "managed";
const DISPLACED_NAME: &str = "displaced";
const CHILD_NAME: &str = "child";
const STATE_NAME: &str = "state.json";
const SENTINEL_NAME: &str = "sentinel";
const SENTINEL_VALUE: &str = "replacement";
const ORIGINAL_STATE: u8 = 1;
const REPLACEMENT_STATE: u8 = 2;

#[test]
fn captured_directories_remain_valid_until_replaced() {
    let root = tempfile::tempdir().unwrap();
    let managed = root.path().join("managed");
    crate::paths::directory::PrivateDirectory::ensure(&managed).unwrap();
    let lease = NamespaceLease::capture(&[("managed", &managed)]).unwrap();
    assert!(lease.verify().is_ok());
    assert!(!lease.is_lost());
    assert!(lease.prove_ownership_loss().is_none());
}

#[test]
fn retained_write_rejects_swap_without_touching_replacement() {
    let root = tempfile::tempdir().unwrap();
    let managed = root.path().join(MANAGED_NAME);
    let displaced = root.path().join(DISPLACED_NAME);
    let child = managed.join(CHILD_NAME);
    PrivateDirectory::ensure(&child).unwrap();
    let lease = NamespaceLease::capture(&[(MANAGED_NAME, &managed)]).unwrap();
    let path = child.join(STATE_NAME);
    let entry = lease.file(&path).unwrap();
    std::fs::rename(&managed, &displaced).unwrap();
    PrivateDirectory::ensure(&child).unwrap();
    std::fs::write(managed.join(SENTINEL_NAME), SENTINEL_VALUE).unwrap();

    assert!(crate::paths::atomic_write_entry_guarded(entry, &1_u8, || lease.verify()).is_err());
    assert!(!path.exists());
    assert_eq!(
        std::fs::read_to_string(managed.join(SENTINEL_NAME)).unwrap(),
        SENTINEL_VALUE
    );
}

#[test]
fn retained_directory_creation_never_enters_replacement() {
    let root = tempfile::tempdir().unwrap();
    let managed = root.path().join(MANAGED_NAME);
    let displaced = root.path().join(DISPLACED_NAME);
    PrivateDirectory::ensure(&managed).unwrap();
    let lease = NamespaceLease::capture(&[(MANAGED_NAME, &managed)]).unwrap();
    std::fs::rename(&managed, &displaced).unwrap();
    PrivateDirectory::ensure(&managed).unwrap();
    std::fs::write(managed.join(SENTINEL_NAME), SENTINEL_VALUE).unwrap();

    assert!(lease.ensure_directory(&managed.join(CHILD_NAME)).is_err());
    assert!(!managed.join(CHILD_NAME).exists());
    assert_eq!(
        std::fs::read_to_string(managed.join(SENTINEL_NAME)).unwrap(),
        SENTINEL_VALUE
    );
}

#[test]
fn replacement_permanently_loses_the_lease() {
    let root = tempfile::tempdir().unwrap();
    let managed = root.path().join("managed");
    let displaced = root.path().join("displaced");
    crate::paths::directory::PrivateDirectory::ensure(&managed).unwrap();
    let lease = NamespaceLease::capture(&[("managed", &managed)]).unwrap();
    std::fs::rename(&managed, displaced).unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&managed).unwrap();
    assert!(lease.verify().is_err());
    assert!(lease.is_lost());
    assert!(lease.prove_ownership_loss().is_some());
    assert!(
        lease
            .verify()
            .unwrap_err()
            .to_string()
            .contains(ALREADY_LOST_ERROR)
    );
}

#[test]
fn conditional_json_remove_preserves_a_boundary_replacement() {
    let root = tempfile::tempdir().unwrap();
    let managed = root.path().join(MANAGED_NAME);
    PrivateDirectory::ensure(&managed).unwrap();
    let lease = NamespaceLease::capture(&[(MANAGED_NAME, &managed)]).unwrap();
    let path = managed.join(STATE_NAME);
    lease.write_json(&path, &ORIGINAL_STATE).unwrap();

    let removed = lease.remove_json_if_matches_with_boundary(&path, &ORIGINAL_STATE, || {
        lease.write_json(&path, &REPLACEMENT_STATE)
    });

    assert!(removed.is_err());
    assert_eq!(lease.read_json::<u8>(&path).unwrap(), REPLACEMENT_STATE);
    lease
        .remove_json_if_matches(&path, &REPLACEMENT_STATE)
        .unwrap();
    assert!(lease.file_is_missing(&path).unwrap());
}
