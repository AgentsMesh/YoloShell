use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const FIRST_PENDING_SLOT_INDEX: usize = 0;

#[test]
fn reused_pending_generation_clears_grant_acls() {
    for payload_mode in [PRIVATE_DIRECTORY_MODE, IMMUTABLE_DIRECTORY_MODE] {
        assert_reused_payload_clears_acl(payload_mode);
    }
}

fn assert_reused_payload_clears_acl(payload_mode: u32) {
    let base = tempfile::tempdir().unwrap();
    let root_path = base.path().join(CONFIG_GENERATIONS_DIRECTORY);
    let root = GenerationRoot::ensure(&root_path).unwrap();
    let slot = root_path.join(format!(
        "{PENDING_GENERATION_PREFIX}{FIRST_PENDING_SLOT_INDEX}"
    ));
    std::fs::create_dir(&slot).unwrap();
    std::fs::set_permissions(
        &slot,
        std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    crate::acl_fixture::add_read_grant(&slot);
    let payload = root_path.join(format!(
        "{PENDING_GENERATION_PREFIX}{FIRST_PENDING_SLOT_INDEX}{PENDING_GENERATION_PAYLOAD_SUFFIX}"
    ));
    std::fs::create_dir(&payload).unwrap();
    std::fs::set_permissions(&payload, std::fs::Permissions::from_mode(payload_mode)).unwrap();
    crate::acl_fixture::add_read_grant(&payload);
    let original_inode = std::fs::metadata(&payload).unwrap().ino();
    assert!(
        crate::private_acl::require_empty(&std::fs::File::open(&payload).unwrap(), &payload)
            .is_err()
    );

    let pending = PendingDirectory::create(&root).unwrap();

    assert_eq!(pending.path, payload);
    assert_eq!(pending.directory.metadata().unwrap().ino(), original_inode);
    crate::private_acl::require_empty(&pending.directory, &pending.path).unwrap();
    crate::private_acl::require_empty(&std::fs::File::open(&slot).unwrap(), &slot).unwrap();
}
