use super::*;
use std::ffi::CString;
use std::os::unix::fs::PermissionsExt;

const SLOT_NAME: &str = "empty-remove-fault-slot";
const DISPLACED_NAME: &str = "displaced-empty-remove-fault-slot";
const CHILD_NAME: &str = "blocking-child";
const CHILD_BYTES: &[u8] = b"blocking-child";
const UNSAFE_DIRECTORY_MODE: u32 = 0o755;

#[derive(Clone, Copy, Debug)]
enum RemoveCase {
    Busy,
    Missing,
    NonEmpty,
    ReplacedAtFence,
    Removed,
    Unsafe,
    UnsafeMode,
}

#[test]
fn empty_remove_matrix_requires_lock_identity_and_two_empty_proofs() {
    for case in [
        RemoveCase::Busy,
        RemoveCase::Missing,
        RemoveCase::NonEmpty,
        RemoveCase::ReplacedAtFence,
        RemoveCase::Removed,
        RemoveCase::Unsafe,
        RemoveCase::UnsafeMode,
    ] {
        run_case(case);
    }
}

fn run_case(case: RemoveCase) {
    let temporary = tempfile::tempdir().unwrap();
    std::fs::set_permissions(
        temporary.path(),
        std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    let root = PrivateDirectory::open(temporary.path()).unwrap();
    let path = temporary.path().join(SLOT_NAME);
    let displaced = temporary.path().join(DISPLACED_NAME);
    let expected = root.ensure_descendant(&path).unwrap();
    let name = CString::new(SLOT_NAME).unwrap();
    let held = matches!(case, RemoveCase::Busy)
        .then(|| lifecycle_lock::SocketNamespaceLifecycleLock::acquire(&root).unwrap());
    match case {
        RemoveCase::Missing => std::fs::remove_dir(&path).unwrap(),
        RemoveCase::NonEmpty => std::fs::write(path.join(CHILD_NAME), CHILD_BYTES).unwrap(),
        RemoveCase::Unsafe => {
            std::fs::rename(&path, &displaced).unwrap();
            std::fs::write(&path, CHILD_BYTES).unwrap();
        }
        RemoveCase::UnsafeMode => std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(UNSAFE_DIRECTORY_MODE),
        )
        .unwrap(),
        RemoveCase::Busy | RemoveCase::ReplacedAtFence | RemoveCase::Removed => {}
    }

    let removed = empty_remove::owned_with_test_hook(&root, &expected, &name, || {
        if matches!(case, RemoveCase::ReplacedAtFence) {
            std::fs::rename(&path, &displaced)?;
            root.ensure_descendant(&path)?;
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(removed, matches!(case, RemoveCase::Removed), "{case:?}");
    assert_eq!(
        path.exists(),
        !matches!(case, RemoveCase::Missing | RemoveCase::Removed)
    );
    drop(held);
}
