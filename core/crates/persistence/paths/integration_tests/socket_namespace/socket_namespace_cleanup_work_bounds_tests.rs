use super::test_hooks::{self, Point};
use super::*;
use std::cell::Cell;
use std::os::unix::{fs::PermissionsExt, net::UnixListener};
use std::rc::Rc;
use std::time::Duration;

const SLOW_IO_COMPLETION_MARGIN: Duration = Duration::from_millis(25);

#[test]
fn registered_stage_cleanup_finishes_after_slow_filesystem_completion() {
    let (_home, _root, owner) = fixture();
    let directory = owner.directory().unwrap();
    create_released_stage_leases(&owner, &directory);
    let (_delay, observed) = slow_completion(Point::AfterRegisteredStageChunk);

    assert_eq!(owner.release().unwrap(), NamespaceRelease::Removed);

    assert!(observed.get());
    assert!(!directory.path().exists());
}

#[test]
fn verified_stale_endpoints_are_removed_after_slow_inventory_completion() {
    let (_home, _root, owner) = fixture();
    let directory = owner.directory().unwrap();
    let socket = owner.worker_socket(Uuid::new_v4()).unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    drop(listener);
    let (_delay, observed) = slow_completion(Point::AfterEndpointInventory);

    assert_eq!(owner.release().unwrap(), NamespaceRelease::Removed);

    assert!(observed.get());
    assert!(!socket.exists());
    assert!(!directory.path().exists());
}

#[test]
fn certified_orphan_bookkeeping_finishes_after_slow_filesystem_completion() {
    let (_home, root, mut owner) = fixture();
    let directory = owner.directory().unwrap();
    create_released_stage_leases(&owner, &directory);
    let authority = super::super::root_visit::StageAuthority::capture(
        &directory,
        directory.path().file_name().unwrap(),
    )
    .unwrap()
    .unwrap();
    owner.preserve_on_release();
    drop(owner);
    let root = PrivateDirectory::open(root.path()).unwrap();
    let mut lifecycle = lifecycle_lock::SocketNamespaceLifecycleLock::acquire(&root).unwrap();
    let (_delay, observed) = slow_completion(Point::AfterOrphanStageChunk);

    assert!(authority.cleanup_empty(&root, &mut lifecycle).unwrap());

    assert!(observed.get());
    assert!(!directory.path().exists());
}

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, SocketNamespace) {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(ISOLATED_TEST_SOCKET_PARENT).unwrap();
    let root_path = root.path().canonicalize().unwrap();
    PrivateDirectory::ensure(&root_path).unwrap();
    let paths = Paths::new_with_socket_root(home.path(), root_path);
    let owner = paths.retain_socket_namespace().unwrap();
    (home, root, owner)
}

fn create_released_stage_leases(owner: &SocketNamespace, directory: &PrivateDirectory) {
    for endpoint in [
        owner.host_socket().unwrap(),
        owner.worker_socket(Uuid::new_v4()).unwrap(),
    ] {
        let name = entry_c_string(endpoint.file_name().unwrap()).unwrap();
        let target = socket_stage::Target::new(directory.path(), &name).unwrap();
        drop(socket_stage::Lease::acquire_immediate_for_test(directory, &target).unwrap());
    }
}

fn slow_completion(point: Point) -> (test_hooks::HookGuard, Rc<Cell<bool>>) {
    let observed = Rc::new(Cell::new(false));
    let completed = Rc::clone(&observed);
    let guard = test_hooks::install(point, move || {
        std::thread::sleep(SOCKET_STAGE_LEASE_TIMEOUT + SLOW_IO_COMPLETION_MARGIN);
        completed.set(true);
    });
    (guard, observed)
}
