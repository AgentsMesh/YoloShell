use super::super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use tempfile::tempdir;

const OPEN_DIRECTORY_MODE: u32 = 0o755;
const UNSAFE_DIRECTORY_MODE: u32 = 0o777;

#[test]
fn socket_directory_is_stable_and_scoped_by_home() {
    let paths = Paths::new("/Users/example/.yolo");
    assert_eq!(
        paths.socket_dir().unwrap(),
        Paths::new("/Users/example/.yolo").socket_dir().unwrap()
    );
    assert_ne!(
        paths.socket_dir().unwrap(),
        Paths::new("/Users/example/.other-yolo")
            .socket_dir()
            .unwrap()
    );
}

#[test]
fn home_symlink_is_rejected_without_chmod_or_child_creation() {
    let root = tempdir().unwrap();
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    set_mode(&outside, OPEN_DIRECTORY_MODE);
    let home = root.path().join("home");
    symlink(&outside, &home).unwrap();
    assert!(Paths::new(&home).ensure_base().is_err());
    assert_eq!(mode(&outside), OPEN_DIRECTORY_MODE);
    assert!(!outside.join(RUNTIME_DIRECTORY).exists());
}

#[test]
fn symlinked_sessions_and_session_directories_are_rejected() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let paths = Paths::new(&home);
    paths.ensure_base().unwrap();
    fs::remove_dir(paths.sessions_dir()).unwrap();
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    set_mode(&outside, PRIVATE_DIRECTORY_MODE);
    symlink(&outside, paths.sessions_dir()).unwrap();
    assert!(paths.ensure_base().is_err());
    assert!(fs::read_dir(&outside).unwrap().next().is_none());

    fs::remove_file(paths.sessions_dir()).unwrap();
    fs::create_dir(paths.sessions_dir()).unwrap();
    set_mode(&paths.sessions_dir(), PRIVATE_DIRECTORY_MODE);
    let terminal_id = Uuid::new_v4();
    symlink(&outside, paths.session_dir(terminal_id)).unwrap();
    assert!(paths.ensure_session(terminal_id).is_err());
    assert!(!outside.join(TERMINAL_DIRECTORY).exists());
}

#[test]
fn unsafe_or_non_directory_home_is_rejected_without_mutation() {
    let root = tempdir().unwrap();
    let unsafe_home = root.path().join("unsafe");
    fs::create_dir(&unsafe_home).unwrap();
    set_mode(&unsafe_home, UNSAFE_DIRECTORY_MODE);
    assert!(Paths::new(&unsafe_home).ensure_base().is_err());
    assert_eq!(mode(&unsafe_home), UNSAFE_DIRECTORY_MODE);

    let file_home = root.path().join("file-home");
    fs::write(&file_home, b"unchanged").unwrap();
    assert!(Paths::new(&file_home).ensure_base().is_err());
    assert_eq!(fs::read(&file_home).unwrap(), b"unchanged");
}

#[test]
fn an_ancestor_symlink_is_never_followed() {
    let root = tempdir().unwrap();
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let alias = root.path().join("alias");
    symlink(&outside, &alias).unwrap();
    let home = alias.join("home");
    assert!(Paths::new(&home).ensure_base().is_err());
    assert!(!outside.join("home").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn private_directories_remove_inherited_acls_and_reject_later_acls() {
    use std::process::Command;
    const CHMOD: &str = "/bin/chmod";
    const ALLOW_READ_ACL: &str = "everyone allow read";
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    fs::create_dir(&home).unwrap();
    set_mode(&home, PRIVATE_DIRECTORY_MODE);
    assert!(
        Command::new(CHMOD)
            .args(["+a", ALLOW_READ_ACL])
            .arg(&home)
            .status()
            .unwrap()
            .success()
    );
    assert!(directory::PrivateDirectory::open(&home).is_err());

    Paths::new(&home).ensure_base().unwrap();
    crate::private_acl::require_empty(&fs::File::open(&home).unwrap(), &home).unwrap();

    assert!(
        Command::new(CHMOD)
            .args(["+a", ALLOW_READ_ACL])
            .arg(&home)
            .status()
            .unwrap()
            .success()
    );
    assert!(directory::PrivateDirectory::open(&home).is_err());
    let entries_before = fs::read_dir(&home).unwrap().count();
    assert!(Paths::new(&home).ensure_base().is_err());
    assert!(directory::PrivateDirectory::open(&home).is_err());
    assert_eq!(fs::read_dir(&home).unwrap().count(), entries_before);
}

fn set_mode(path: &Path, value: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(value)).unwrap();
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & PERMISSION_MODE_MASK
}
