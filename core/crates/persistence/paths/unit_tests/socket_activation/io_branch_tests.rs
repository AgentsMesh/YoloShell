use super::*;
use crate::paths::constants::{PRIVATE_DIRECTORY_MODE, SOCKET_ROOT_PREFIX};
use std::fs;
use std::os::unix::fs::PermissionsExt;

const BLOCKING_ENTRY_NAME: &str = "blocking-entry";
const BLOCKING_ENTRY_BYTES: &[u8] = b"preserve";
const BLOCKED_CHILD_NAME: &str = "child";
const MISSING_ENTRY_NAME: &str = "missing.sock";
const TEST_ROOT_PREFIX: &str = "yi-";

#[test]
fn inspection_distinguishes_absence_from_non_directory_traversal() {
    let (_temporary, directory) = fixture();
    let missing = CString::new(MISSING_ENTRY_NAME).unwrap();
    assert_eq!(inspect_at(&directory, &missing, false).unwrap(), None);

    let blocker = directory.path().join(BLOCKING_ENTRY_NAME);
    fs::write(&blocker, BLOCKING_ENTRY_BYTES).unwrap();
    let blocked = CString::new(format!("{BLOCKING_ENTRY_NAME}/{BLOCKED_CHILD_NAME}")).unwrap();
    assert!(inspect_at(&directory, &blocked, false).is_err());
    assert_eq!(fs::read(blocker).unwrap(), BLOCKING_ENTRY_BYTES);
}

#[tokio::test(flavor = "current_thread")]
async fn mutation_helpers_surface_real_kernel_failures() {
    let (_temporary, directory) = fixture();
    let missing = CString::new(MISSING_ENTRY_NAME).unwrap();
    assert!(unlink_at(&directory, &missing).is_err());
    assert!(chmod_at(&directory, &missing).is_err());

    let endpoint = directory.path().join(MISSING_ENTRY_NAME);
    let target = SocketTarget::new(directory.try_clone().unwrap(), &endpoint).unwrap();
    let stage = StageEntry::vacant(&target, socket_stage::Kind::Bind)
        .await
        .unwrap();
    assert!(link_at(&target, &stage).is_err());
    assert!(rename_exclusive(&stage, &target).is_err());
    assert!(!endpoint.exists());
    assert!(!stage.path().exists());
}

fn fixture() -> (tempfile::TempDir, PrivateDirectory) {
    let parent = Path::new(SOCKET_ROOT_PREFIX).parent().unwrap();
    let temporary = tempfile::Builder::new()
        .prefix(TEST_ROOT_PREFIX)
        .tempdir_in(parent)
        .unwrap();
    fs::set_permissions(
        temporary.path(),
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    let directory = PrivateDirectory::open(temporary.path()).unwrap();
    (temporary, directory)
}
