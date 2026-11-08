use super::*;
use crate::paths::constants::{HOST_SOCKET_FILE_NAME, PRIVATE_FILE_MODE, SOCKET_ROOT_PREFIX};
use crate::paths::directory::PrivateDirectory;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use tempfile::{Builder, TempDir};

const COLLIDING_DIGEST_CHARACTER: char = 'f';
const FALLBACK_COLLIDING_DIGEST_CHARACTER: char = 'e';
const TEST_TEMP_PREFIX: &str = "yl-";

#[tokio::test(flavor = "current_thread")]
async fn async_lease_contention_yields_the_current_thread() {
    let (_temporary, directory, target) = fixture();
    let first = Lease::acquire_immediate_for_test(&directory, &target).unwrap();
    let release = tokio::spawn(async move {
        tokio::task::yield_now().await;
        drop(first);
    });

    let second = Lease::acquire(&directory, &target).await.unwrap();

    release.await.unwrap();
    drop(second);
}

#[test]
fn live_target_lease_fences_stage_and_lifecycle_cleanup() {
    let (_temporary, directory, target) = fixture();
    let lease = Lease::acquire_immediate_for_test(&directory, &target).unwrap();
    let (_, stage) = target.stage(Kind::Bind).unwrap();
    let listener = UnixListener::bind(&stage).unwrap();
    fs::set_permissions(&stage, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let stage_identity = fs::symlink_metadata(&stage).unwrap().ino();
    let lease_identity = fs::symlink_metadata(target.lease().unwrap().1)
        .unwrap()
        .ino();
    let lifecycle_identity = fs::symlink_metadata(target.lifecycle_path()).unwrap().ino();

    let live_cleanup = cleanup_key(&directory, target.key()).unwrap();
    assert!(matches!(live_cleanup, Cleanup::Busy), "{live_cleanup:?}");
    assert_eq!(fs::symlink_metadata(&stage).unwrap().ino(), stage_identity);
    assert_eq!(
        fs::symlink_metadata(target.lease().unwrap().1)
            .unwrap()
            .ino(),
        lease_identity
    );
    assert_eq!(
        fs::symlink_metadata(target.lifecycle_path()).unwrap().ino(),
        lifecycle_identity
    );

    drop(listener);
    drop(lease);
    assert!(matches!(
        cleanup_key(&directory, target.key()).unwrap(),
        Cleanup::Mutated
    ));
    assert!(matches!(
        cleanup_lifecycle(&directory).unwrap(),
        Cleanup::Mutated
    ));
    assert!(!stage.exists());
    assert!(!target.lease().unwrap().1.exists());
    assert!(!target.lifecycle_path().exists());
}

#[test]
fn full_digest_collision_preserves_the_first_lease_identity_and_record() {
    let (_temporary, directory, target) = fixture();
    drop(Lease::acquire_immediate_for_test(&directory, &target).unwrap());
    let lease_path = target.lease().unwrap().1;
    let original_identity = fs::symlink_metadata(&lease_path).unwrap().ino();
    let original_record = fs::read(&lease_path).unwrap();
    let colliding = colliding_target(&target);

    assert_eq!(colliding.key(), target.key());
    assert!(Lease::acquire_immediate_for_test(&directory, &colliding).is_err());
    assert_eq!(
        fs::symlink_metadata(&lease_path).unwrap().ino(),
        original_identity
    );
    assert_eq!(fs::read(&lease_path).unwrap(), original_record);
}

fn fixture() -> (TempDir, PrivateDirectory, Target) {
    let parent = Path::new(SOCKET_ROOT_PREFIX).parent().unwrap();
    let temporary = Builder::new()
        .prefix(TEST_TEMP_PREFIX)
        .tempdir_in(parent)
        .unwrap();
    let directory = PrivateDirectory::ensure(temporary.path()).unwrap();
    let name = CString::new(HOST_SOCKET_FILE_NAME).unwrap();
    let target = Target::new(directory.path(), &name).unwrap();
    (temporary, directory, target)
}

fn colliding_target(target: &Target) -> Target {
    let mut digest = target.full_digest.clone();
    let original = digest.as_bytes()[KEY_HEX_LENGTH] as char;
    let replacement = if original == COLLIDING_DIGEST_CHARACTER {
        FALLBACK_COLLIDING_DIGEST_CHARACTER
    } else {
        COLLIDING_DIGEST_CHARACTER
    };
    digest.replace_range(KEY_HEX_LENGTH..=KEY_HEX_LENGTH, &replacement.to_string());
    Target::from_digest(target.parent.as_path(), target.effective_user_id, digest).unwrap()
}

#[path = "socket_stage_branch_matrix.rs"]
mod branch_matrix;
#[path = "socket_stage_coverage_matrix.rs"]
mod coverage_matrix;
#[path = "socket_stage_lifecycle_contention_tests.rs"]
mod lifecycle_contention_tests;
#[path = "socket_stage_probe_target_branch_tests.rs"]
mod probe_target_branch_tests;
