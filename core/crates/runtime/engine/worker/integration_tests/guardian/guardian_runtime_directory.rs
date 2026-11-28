use super::*;
use crate::paths::socket_namespace::NamespaceRelease;
use std::os::unix::fs::PermissionsExt;

const GUARDIAN_SOCKET_MODE: u32 = 0o600;
const UNKNOWN_NAMESPACE_ENTRY: &str = "unproven-fixture-entry";

#[test]
fn guardian_runtime_directory_requires_explicit_creator_completion() {
    let (_owner, directory, paths) = fixture();
    initialize_namespace(&paths);
    let result = std::panic::catch_unwind(move || drop(directory));
    assert!(result.is_err());
    assert!(paths.home().is_dir());
    release_namespace(&paths);
}

#[test]
fn guardian_runtime_directory_preserves_home_during_early_unwind() {
    let (_owner, directory, paths) = fixture();
    initialize_namespace(&paths);
    let result = std::panic::catch_unwind(move || {
        let _owned = directory;
        std::panic::panic_any(GUARDIAN_HOME_UNSETTLED_DETAIL);
    });
    assert!(result.is_err());
    assert!(paths.home().is_dir());
    release_namespace(&paths);
}

#[test]
fn guardian_runtime_directory_deletes_only_after_explicit_namespace_release() {
    let (_owner, directory, paths) = fixture();
    let namespace = initialize_namespace(&paths);
    assert!(namespace.is_dir());
    directory.close(&paths).unwrap();
    assert!(!namespace.exists());
    assert!(!paths.home().exists());
}

#[test]
fn guardian_runtime_directory_removes_stale_guardian_socket_before_home() {
    let (_owner, directory, paths) = fixture();
    let mut namespace = paths.retain_socket_namespace().unwrap();
    let namespace_path = namespace.directory().unwrap().path().to_path_buf();
    let socket = paths.custody_guardian_socket(Uuid::new_v4()).unwrap();
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    std::fs::set_permissions(
        &socket,
        std::fs::Permissions::from_mode(GUARDIAN_SOCKET_MODE),
    )
    .unwrap();
    drop(listener);
    namespace.preserve_on_release();
    drop(namespace);
    assert!(socket.exists());

    directory.close(&paths).unwrap();

    assert!(!socket.exists());
    assert!(!namespace_path.exists());
    assert!(!paths.home().exists());
}

#[test]
fn guardian_runtime_directory_keeps_home_when_namespace_is_busy() {
    let (_owner, directory, paths) = fixture();
    let retained = paths.retain_socket_namespace().unwrap();
    let error = directory.close(&paths).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(GUARDIAN_NAMESPACE_RELEASE_DETAIL)
    );
    assert!(paths.home().is_dir());
    assert_eq!(retained.release().unwrap(), NamespaceRelease::Removed);
}

#[test]
fn guardian_runtime_directory_keeps_home_when_namespace_is_preserved() {
    let (_owner, directory, paths) = fixture();
    let namespace = initialize_namespace(&paths);
    let unknown = namespace.join(UNKNOWN_NAMESPACE_ENTRY);
    std::fs::write(&unknown, []).unwrap();
    let error = directory.close(&paths).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(GUARDIAN_NAMESPACE_RELEASE_DETAIL)
    );
    assert!(paths.home().is_dir());
    assert!(unknown.exists());
    std::fs::remove_file(unknown).unwrap();
    release_namespace(&paths);
}

#[test]
fn guardian_runtime_directory_cleanup_failure_keeps_displaced_evidence() {
    let (owner, directory, paths) = fixture();
    initialize_namespace(&paths);
    let target = owner.path().join(FIXTURE_HOME_COMPONENT);
    std::fs::rename(directory.path(), &target).unwrap();
    assert!(directory.close(&paths).is_err());
    assert!(target.is_dir());
    // Restore only this test's moved home so the retained authority can clean its namespace.
    std::fs::rename(&target, paths.home()).unwrap();
    release_namespace(&paths);
}

pub(super) fn fixture() -> (tempfile::TempDir, GuardianRuntimeDirectory, Paths) {
    let owner = tempfile::tempdir().unwrap();
    let directory = GuardianRuntimeDirectory {
        directory: Some(tempfile::tempdir_in(owner.path()).unwrap()),
        cleanup_attempted: false,
    };
    let paths = Paths::new(directory.path());
    (owner, directory, paths)
}

fn initialize_namespace(paths: &Paths) -> PathBuf {
    let mut namespace = paths.retain_socket_namespace().unwrap();
    let path = namespace.directory().unwrap().path().to_path_buf();
    namespace.preserve_on_release();
    drop(namespace);
    path
}

fn release_namespace(paths: &Paths) {
    assert_eq!(
        paths.retain_socket_namespace().unwrap().release().unwrap(),
        NamespaceRelease::Removed
    );
}
