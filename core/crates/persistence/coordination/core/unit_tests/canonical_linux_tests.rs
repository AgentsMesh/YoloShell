use super::*;

const ALIAS: &str = "alias";
const ORIGINAL: &str = "original";
const UPPER: &str = "Missing";
const LOWER: &str = "missing";

#[test]
fn existing_symlink_alias_and_future_suffix_share_the_same_coordination_path() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join(ORIGINAL);
    std::fs::create_dir(&original).unwrap();
    let alias = root.path().join(ALIAS);
    std::os::unix::fs::symlink(&original, &alias).unwrap();
    assert_eq!(canonical_logical_path(&alias).unwrap(), original);
    assert_eq!(
        canonical_logical_path(&alias.join(UPPER)).unwrap(),
        original.join(UPPER)
    );
}

#[test]
fn linux_paths_preserve_case_and_identity_after_creation() {
    let root = tempfile::tempdir().unwrap();
    let upper = root.path().join(UPPER);
    let lower = root.path().join(LOWER);
    let before = canonical_logical_path(&upper).unwrap();
    assert_ne!(before, canonical_logical_path(&lower).unwrap());
    std::fs::create_dir(&upper).unwrap();
    assert_eq!(before, canonical_logical_path(&upper).unwrap());
}

#[test]
fn non_directory_ancestors_do_not_authorize_future_paths() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join(ORIGINAL);
    std::fs::write(&file, []).unwrap();
    assert!(canonical_logical_path(&file.join(UPPER)).is_err());
}
