use super::*;
use crate::namespace_lease::NamespaceLease;
use crate::paths::directory::PrivateDirectory;
use std::fs;

const MANAGED_DIRECTORY: &str = "managed";
const DISPLACED_DIRECTORY: &str = "displaced";
const TARGET_FILE: &str = "integration.zsh";
const SENTINEL_FILE: &str = "replacement-sentinel";
const ORIGINAL_BYTES: &[u8] = b"installed script\n";
const REPLACEMENT_BYTES: &[u8] = b"replacement script\n";
const SENTINEL_BYTES: &[u8] = b"replacement namespace";

#[test]
fn final_namespace_custody_fence_rejects_replacement_before_install_or_exchange() {
    for original in [None, Some(ORIGINAL_BYTES)] {
        assert_final_namespace_replacement_is_rejected(original);
    }
}

fn assert_final_namespace_replacement_is_rejected(original: Option<&[u8]>) {
    let root = tempfile::tempdir().unwrap();
    let managed = root.path().join(MANAGED_DIRECTORY);
    let displaced = root.path().join(DISPLACED_DIRECTORY);
    PrivateDirectory::ensure(&managed).unwrap();
    let lease = NamespaceLease::capture(&[(MANAGED_DIRECTORY, &managed)]).unwrap();
    let target = managed.join(TARGET_FILE);
    if let Some(bytes) = original {
        write_private_bytes_guarded(lease.file(&target).unwrap(), bytes, u64::MAX, || {
            lease.verify()
        })
        .unwrap();
    }

    let result = write_private_bytes_guarded_with_commit_boundary(
        lease.file(&target).unwrap(),
        REPLACEMENT_BYTES,
        u64::MAX,
        || lease.verify(),
        || {
            fs::rename(&managed, &displaced)?;
            PrivateDirectory::ensure(&managed)?;
            fs::write(managed.join(SENTINEL_FILE), SENTINEL_BYTES)?;
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(lease.is_lost());
    assert!(!target.exists());
    assert_eq!(
        fs::read(managed.join(SENTINEL_FILE)).unwrap(),
        SENTINEL_BYTES
    );
    let displaced_target = displaced.join(TARGET_FILE);
    match original {
        Some(bytes) => assert_eq!(fs::read(displaced_target).unwrap(), bytes),
        None => assert!(!displaced_target.exists()),
    }
}
