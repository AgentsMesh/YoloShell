use super::*;

const FILESYSTEM_ROOT: &str = "/";

#[test]
fn logical_keys_are_stable_and_scoped() {
    let home = tempfile::tempdir().unwrap();
    let first = lock_path(CoordinationScope::HostHome, home.path(), None).unwrap();
    let repeated = lock_path(CoordinationScope::HostHome, home.path(), None).unwrap();
    let endpoint = lock_path(CoordinationScope::HostEndpoint, home.path(), None).unwrap();
    let worker = lock_path(CoordinationScope::Worker, home.path(), Some("session")).unwrap();
    assert_eq!(first, repeated);
    assert_ne!(first, endpoint);
    assert_ne!(first, worker);
    assert!(first.parent().unwrap().is_dir());
}

#[test]
fn discriminator_and_logical_path_change_the_key() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let one = lock_path(CoordinationScope::Worker, first.path(), Some("one")).unwrap();
    let two = lock_path(CoordinationScope::Worker, first.path(), Some("two")).unwrap();
    let other = lock_path(CoordinationScope::Worker, second.path(), Some("one")).unwrap();
    assert_ne!(one, two);
    assert_ne!(one, other);
}

#[test]
fn managed_namespace_cannot_contain_coordination_storage() {
    let root = coordination_root();
    assert!(lock_path(CoordinationScope::HostHome, &root, None).is_err());
    assert!(lock_path(CoordinationScope::HostHome, root.parent().unwrap(), None).is_err());
}

#[test]
fn a_lock_path_without_a_parent_fails_before_acquiring_authority() {
    assert!(
        CoordinationLock::try_acquire(Path::new(FILESYSTEM_ROOT), MAX_COORDINATION_LOCK_BYTES)
            .is_err()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_system_aliases_share_coordination_keys() {
    let alias = Path::new("/tmp/yolo-coordination-alias-test");
    let physical = Path::new("/private/tmp/yolo-coordination-alias-test");
    assert_eq!(
        lock_path(CoordinationScope::HostHome, alias, None).unwrap(),
        lock_path(CoordinationScope::HostHome, physical, None).unwrap()
    );

    let alias = Path::new("/var/yolo-coordination-alias-test");
    let physical = Path::new("/private/var/yolo-coordination-alias-test");
    assert_eq!(
        lock_path(CoordinationScope::Worker, alias, Some("session")).unwrap(),
        lock_path(CoordinationScope::Worker, physical, Some("session")).unwrap()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn coordination_alias_cannot_bypass_overlap_check() {
    let root = coordination_root();
    let alias = Path::new("/tmp").join(root.file_name().unwrap());
    assert!(lock_path(CoordinationScope::HostHome, &alias, None).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_firmlink_spellings_share_coordination_keys() {
    let visible = Path::new("/Users");
    let physical = Path::new("/System/Volumes/Data/Users");
    assert_eq!(
        lock_path(CoordinationScope::HostHome, visible, None).unwrap(),
        lock_path(CoordinationScope::HostHome, physical, None).unwrap()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_existing_case_aliases_share_coordination_keys() {
    let root = tempfile::tempdir().unwrap();
    let stored = root.path().join("MixedCase");
    std::fs::create_dir(&stored).unwrap();
    let alias = root.path().join("mixedcase");
    if alias.exists() {
        assert_eq!(
            lock_path(CoordinationScope::HostHome, &stored, None).unwrap(),
            lock_path(CoordinationScope::HostHome, &alias, None).unwrap()
        );
    }
}
