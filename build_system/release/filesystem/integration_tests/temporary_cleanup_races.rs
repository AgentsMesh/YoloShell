use crate::TemporaryDirectory;
use std::fs;
use std::os::unix::fs::PermissionsExt;

const CHILD_NAME: &str = "original-child";
const OWNER_PREFIX: &str = "yolo-cleanup-test-";
const MOVED_NAME: &str = "moved";
const SENTINEL_NAME: &str = "foreign-data";
const FOREIGN_BYTES: &[u8] = b"preserve the concurrent namespace replacement";
const READ_ONLY_MODE: u32 = 0o400;
const MODE_MASK: u32 = 0o777;
const CONFLICT_NAME: &str = "concurrent-original-name";
const RETIRED_NAME: &str = "claimed-replacement";
const CONFLICT_BYTES: &[u8] = b"independent newer occupant";

#[test]
fn replacement_between_validation_and_root_claim_is_restored_without_mutation() {
    let saved = tempfile::tempdir().unwrap();
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    owner.create_child(CHILD_NAME).unwrap();
    let root = owner.path();
    let original = saved.path().join(MOVED_NAME);
    let result = owner.cleanup_with(|| {
        fs::rename(&root, &original)?;
        fs::create_dir(&root)?;
        fs::write(root.join(SENTINEL_NAME), FOREIGN_BYTES)?;
        fs::set_permissions(
            root.join(SENTINEL_NAME),
            fs::Permissions::from_mode(READ_ONLY_MODE),
        )?;
        Ok(())
    });
    assert!(format!("{:#}", result.unwrap_err()).contains("cleanup encountered a replacement"));
    drop(owner);
    assert_eq!(fs::read(root.join(SENTINEL_NAME)).unwrap(), FOREIGN_BYTES);
    assert_eq!(
        fs::metadata(root.join(SENTINEL_NAME))
            .unwrap()
            .permissions()
            .mode()
            & MODE_MASK,
        READ_ONLY_MODE
    );
    assert!(original.join(CHILD_NAME).is_dir());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn child_replacement_at_root_claim_preserves_foreign_child_in_retired_root() {
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    owner.create_child(CHILD_NAME).unwrap();
    let root = owner.path();
    let result = owner.cleanup_with(|| {
        fs::rename(root.join(CHILD_NAME), root.join(MOVED_NAME))?;
        fs::create_dir(root.join(CHILD_NAME))?;
        fs::write(root.join(CHILD_NAME).join(SENTINEL_NAME), FOREIGN_BYTES)?;
        Ok(())
    });
    assert!(result.is_err());
    let preserved = owner.path();
    drop(owner);
    assert_ne!(preserved, root);
    assert_eq!(
        fs::read(preserved.join(CHILD_NAME).join(SENTINEL_NAME)).unwrap(),
        FOREIGN_BYTES
    );
    assert!(preserved.join(MOVED_NAME).is_dir());
    fs::remove_dir_all(preserved).unwrap();
}

#[test]
fn partial_owned_construction_cleans_up_but_failed_child_capture_preserves_evidence() {
    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    let root = owner.path();
    owner.create_child(CHILD_NAME).unwrap();
    drop(owner);
    assert!(!root.exists());

    let mut owner = TemporaryDirectory::create(OWNER_PREFIX).unwrap();
    let root = owner.path();
    let child = owner.create_child(CHILD_NAME).unwrap();
    fs::write(child.display_path().join(SENTINEL_NAME), FOREIGN_BYTES).unwrap();
    assert!(owner.create_child(CHILD_NAME).is_err());
    drop(owner);
    assert_eq!(
        fs::read(root.join(CHILD_NAME).join(SENTINEL_NAME)).unwrap(),
        FOREIGN_BYTES
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn conflicting_original_name_cannot_be_overwritten_while_restoring_a_bad_claim() {
    use crate::DirectoryHandle;
    use crate::temporary_retirement::confirm_or_restore;
    use std::ffi::CString;

    let root = tempfile::tempdir().unwrap();
    for name in [CHILD_NAME, CONFLICT_NAME, RETIRED_NAME] {
        fs::create_dir(root.path().join(name)).unwrap();
    }
    let retired_path = root.path().join(RETIRED_NAME);
    fs::write(retired_path.join(SENTINEL_NAME), FOREIGN_BYTES).unwrap();
    fs::write(
        root.path().join(CONFLICT_NAME).join(SENTINEL_NAME),
        CONFLICT_BYTES,
    )
    .unwrap();
    let parent = DirectoryHandle::open(root.path()).unwrap();
    let expected = DirectoryHandle::open_at(&parent, CHILD_NAME).unwrap();
    let confirmation = parent.verify_child(RETIRED_NAME, &expected);
    let error = confirm_or_restore(
        &parent,
        &CString::new(CONFLICT_NAME).unwrap(),
        &CString::new(RETIRED_NAME).unwrap(),
        &retired_path,
        confirmation,
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains(retired_path.to_str().unwrap()));
    assert_eq!(
        fs::read(retired_path.join(SENTINEL_NAME)).unwrap(),
        FOREIGN_BYTES
    );
    assert_eq!(
        fs::read(root.path().join(CONFLICT_NAME).join(SENTINEL_NAME)).unwrap(),
        CONFLICT_BYTES
    );
}
