use super::*;

const VISIBLE_USERS_PATH: &str = "/Users";
const PHYSICAL_USERS_PATH: &str = "/System/Volumes/Data/Users";
const SOCKET_NAME: &str = "coordination.sock";
const INVALID_CASE_SENSITIVITY: libc::c_long = -1;
const ROOT_PATH: &str = "/";

#[test]
fn firmlink_aliases_have_one_physical_spelling() {
    let physical_path = Path::new(PHYSICAL_USERS_PATH);
    let visible = canonical_logical_path(Path::new(VISIBLE_USERS_PATH)).unwrap();
    let physical = canonical_logical_path(physical_path).unwrap();
    let expected = normalize_existing(
        physical_path.to_path_buf(),
        volume_is_case_sensitive(physical_path).unwrap(),
    );
    assert_eq!(visible, physical);
    assert_eq!(visible, expected);
    assert_eq!(
        physical_firmlink_path(Path::new("/Users/example")),
        Path::new("/System/Volumes/Data/Users/example")
    );
}

#[test]
fn missing_ascii_suffix_obeys_volume_case_semantics() {
    let root = tempfile::tempdir().unwrap();
    let upper = canonical_logical_path(&root.path().join("Missing")).unwrap();
    let lower = canonical_logical_path(&root.path().join("missing")).unwrap();
    let sensitive = volume_is_case_sensitive(root.path()).unwrap();
    assert_eq!(upper == lower, !sensitive);
}

#[test]
fn missing_home_keeps_its_identity_after_creation() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("MissingHome");
    let before = canonical_logical_path(&home).unwrap();
    std::fs::create_dir(&home).unwrap();
    let after = canonical_logical_path(&home).unwrap();
    assert_eq!(before, after);
}

#[test]
fn case_only_rename_keeps_identity_on_insensitive_volumes() {
    let root = tempfile::tempdir().unwrap();
    if volume_is_case_sensitive(root.path()).unwrap() {
        return;
    }
    let upper = root.path().join("RenameHome");
    let lower = root.path().join("renamehome");
    std::fs::create_dir(&upper).unwrap();
    let before = canonical_logical_path(&upper).unwrap();
    std::fs::rename(&upper, &lower).unwrap();
    assert_eq!(before, canonical_logical_path(&lower).unwrap());
}

#[test]
fn missing_component_normalization_is_explicit() {
    let mixed = OsString::from("MiXeD");
    assert_eq!(
        normalize_missing(mixed.clone(), false).unwrap(),
        OsString::from("mixed")
    );
    assert_eq!(
        normalize_missing(mixed, true).unwrap(),
        OsString::from("MiXeD")
    );
    assert!(normalize_missing(OsString::from("naive-\u{00e9}"), false).is_err());
}

#[test]
fn existing_path_normalization_and_case_probe_cover_both_volume_kinds() {
    let mixed = PathBuf::from("/MiXeD");
    assert_eq!(normalize_existing(mixed.clone(), true), mixed);
    assert!(!validate_case_sensitivity(CASE_INSENSITIVE_VOLUME).unwrap());
    assert!(validate_case_sensitivity(CASE_SENSITIVE_VOLUME).unwrap());
    assert!(validate_case_sensitivity(INVALID_CASE_SENSITIVITY).is_err());
}

#[test]
fn invalid_non_directory_ancestor_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file");
    std::fs::write(&file, []).unwrap();
    assert!(deepest_existing_path(file.join("child")).is_err());
    assert_eq!(
        physical_firmlink_path(Path::new(ROOT_PATH)),
        Path::new(ROOT_PATH)
    );
}

#[test]
fn existing_unix_socket_uses_its_parent_for_the_volume_probe() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join(SOCKET_NAME);
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let existing = canonical_logical_path(&socket).unwrap();
    drop(listener);
    std::fs::remove_file(&socket).unwrap();
    assert_eq!(existing, canonical_logical_path(&socket).unwrap());
}
