use super::*;
use std::{fs, os::fd::AsRawFd, process::Command};

const ROOT_ENV: &str = "YOLO_ROOT_INODE_LOCK_TEST_PATH";
const EXPECTED_ENV: &str = "YOLO_ROOT_INODE_LOCK_TEST_EXPECTED";
const LOCKED: &str = "locked";
const UNLOCKED: &str = "unlocked";
const SOURCE: &str = "owned";
const TARGET: &str = "moved";
const OTHER_ROOT: &str = "other-root";

fn root() -> (tempfile::TempDir, PrivateDirectory) {
    let temporary = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(&temporary.path().canonicalize().unwrap()).unwrap();
    (temporary, directory)
}

fn kernel_probe(root: &PrivateDirectory, expected: &str) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg(SOCKET_NAMESPACE_ROOT_LOCK_TEST_HELPER_FILTER)
        .arg(crate::paths::constants::PROCESS_FILE_LOCK_TEST_NOCAPTURE_ARGUMENT)
        .env(ROOT_ENV, root.path())
        .env(EXPECTED_ENV, expected);
    let mut child = crate::process::spawn_waitable_test_child(&mut command);
    assert!(child.wait().unwrap().success());
}

#[test]
fn directory_inode_lock_needs_no_regular_lock_file_and_fences_mutations() {
    let (_temporary, root) = root();
    let mut owner = SocketNamespaceLifecycleLock::acquire(&root).unwrap();
    assert!(fs::read_dir(root.path()).unwrap().next().is_none());
    kernel_probe(&root, LOCKED);
    assert!(
        SocketNamespaceLifecycleLock::try_acquire(&root)
            .unwrap()
            .is_none()
    );
    root.ensure_descendant(&root.path().join(SOURCE)).unwrap();
    let source = crate::paths::directory::entry_c_string(std::ffi::OsStr::new(SOURCE)).unwrap();
    let target = crate::paths::directory::entry_c_string(std::ffi::OsStr::new(TARGET)).unwrap();
    assert!(super::root_mutation::rename_exclusive(&root, &source, &target, &mut owner).unwrap());
    assert!(root.path().join(TARGET).exists());
    drop(owner);
    kernel_probe(&root, UNLOCKED);
    assert!(SocketNamespaceLifecycleLock::acquire(&root).is_ok());
}

#[test]
fn root_mutations_require_the_exact_directory_lease() {
    let (_temporary, root) = root();
    let other = root
        .ensure_descendant(&root.path().join(OTHER_ROOT))
        .unwrap();
    other.ensure_descendant(&other.path().join(SOURCE)).unwrap();
    let mut owner = SocketNamespaceLifecycleLock::acquire(&root).unwrap();
    let source = crate::paths::directory::entry_c_string(std::ffi::OsStr::new(SOURCE)).unwrap();
    let target = crate::paths::directory::entry_c_string(std::ffi::OsStr::new(TARGET)).unwrap();
    assert!(super::root_mutation::rename_exclusive(&other, &source, &target, &mut owner).is_err());
    assert!(super::root_mutation::remove_directory(&other, &source, &mut owner).is_err());
    assert!(other.path().join(SOURCE).is_dir());
    assert!(!other.path().join(TARGET).exists());
}

#[test]
fn directory_rename_keeps_inode_authority_for_isolated_root_retirement() {
    let (_temporary, root) = root();
    let original = root.ensure_descendant(&root.path().join(SOURCE)).unwrap();
    let owner = SocketNamespaceLifecycleLock::acquire(&original).unwrap();
    fs::rename(original.path(), root.path().join(TARGET)).unwrap();
    let relocated = root.open_descendant(&root.path().join(TARGET)).unwrap();
    assert!(owner.verify_root(&original).is_err());
    owner.verify_root(&relocated).unwrap();
    kernel_probe(&relocated, LOCKED);
    drop(owner);
    kernel_probe(&relocated, UNLOCKED);
}

#[test]
fn socket_namespace_root_inode_lock_subprocess_helper() {
    let Some(path) = std::env::var_os(ROOT_ENV) else {
        return;
    };
    let file = File::open(path).unwrap();
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    match std::env::var(EXPECTED_ENV).unwrap().as_str() {
        LOCKED => {
            assert_ne!(result, 0);
            assert_eq!(
                std::io::Error::last_os_error().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
        UNLOCKED => {
            assert_eq!(result, 0);
            assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) }, 0);
        }
        _ => panic!("invalid lock probe expectation"),
    }
}
