use super::*;
use tempfile::tempdir;

#[cfg(unix)]
mod read_version_changed_tests;

#[test]
fn paths_are_outside_workspace_and_scoped_by_session() {
    let dir = tempdir().unwrap();
    let paths = Paths::new(dir.path());
    let id = Uuid::new_v4();
    let canonical_dir = platform::platform_path(dir.path());
    assert!(paths.runtime_record(id).starts_with(canonical_dir));
    assert!(
        paths
            .worker_socket(id)
            .unwrap()
            .ends_with(format!("{id}.sock"))
    );
    assert!(
        paths
            .custody_guardian_socket(id)
            .unwrap()
            .ends_with(format!(
                "{CUSTODY_GUARDIAN_SOCKET_PREFIX}{}.sock",
                id.simple()
            ))
    );
    assert_ne!(paths.manifest(id), paths.runtime_record(id));
}

#[test]
fn atomic_json_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("state.json");
    atomic_write_json(&path, &serde_json::json!({"ok": true})).unwrap();
    let value: serde_json::Value = read_json(&path).unwrap();
    assert_eq!(value["ok"], true);
    atomic_write_json(&path, &serde_json::json!({"ok": false})).unwrap();
    let value: serde_json::Value = read_json(&path).unwrap();
    assert_eq!(value["ok"], false);
}

#[test]
fn ensure_session_creates_only_private_durable_locations() {
    let dir = tempdir().unwrap();
    let paths = Paths::new(dir.path().join("a-very-long-home-component"));
    let id = Uuid::new_v4();
    paths.ensure_session(id).unwrap();
    assert!(paths.catalog().starts_with(paths.home()));
    assert!(paths.manifest(id).parent().unwrap().is_dir());
    assert!(paths.terminal_dir(id).is_dir());
    assert!(paths.events_dir(id).is_dir());
    assert!(paths.runtime_dir().is_dir());
    assert!(!paths.socket_dir().unwrap().exists());
    assert!(!paths.worker_socket(id).unwrap().starts_with(paths.home()));
    assert!(validate_home(paths.home()).is_ok());
    assert!(read_json::<serde_json::Value>(&paths.manifest(id)).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(paths.session_dir(id))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
}

#[cfg(unix)]
#[path = "directory/directory_security.rs"]
mod directory_security;
#[cfg(target_os = "macos")]
#[path = "json/json_concurrency_tests.rs"]
mod json_concurrency_tests;
#[cfg(target_os = "macos")]
#[path = "json/json_in_place_tests.rs"]
mod json_in_place_tests;
#[cfg(target_os = "macos")]
#[path = "json/json_instance_matrix_tests.rs"]
mod json_instance_matrix_tests;
#[cfg(unix)]
#[path = "json/json_replacement_tests.rs"]
mod json_replacement_tests;
#[cfg(unix)]
#[path = "json/json_security.rs"]
mod json_security;
#[cfg(target_os = "macos")]
#[path = "private/private_file_digest_tests.rs"]
mod private_file_digest_tests;
#[cfg(target_os = "macos")]
#[path = "private/private_file_identity_tests.rs"]
mod private_file_identity_tests;
#[cfg(target_os = "macos")]
#[path = "private/private_file_missing_tests.rs"]
mod private_file_missing_tests;
#[cfg(target_os = "macos")]
#[path = "private/private_file_remove_tests.rs"]
mod private_file_remove_tests;
#[cfg(target_os = "macos")]
#[path = "private/private_file_swap_tests.rs"]
mod private_file_swap_tests;
#[cfg(target_os = "macos")]
#[path = "private/private_file_temp_race_tests.rs"]
mod private_file_temp_race_tests;
#[cfg(unix)]
#[path = "private/private_file_unmanaged_tests.rs"]
mod private_file_unmanaged_tests;
