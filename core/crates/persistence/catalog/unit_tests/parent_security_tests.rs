use super::Catalog;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const UNSAFE_DIRECTORY_MODE: u32 = 0o777;
const PERMISSION_MODE_MASK: u32 = 0o7777;

#[test]
fn catalog_rejects_a_symlinked_parent_without_creating_an_artifact() {
    let root = tempfile::tempdir().unwrap();
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    set_mode(&outside, PRIVATE_DIRECTORY_MODE);
    let alias = root.path().join("alias");
    symlink(&outside, &alias).unwrap();

    assert!(Catalog::open(&alias.join("catalog.sqlite")).is_err());
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
}

#[test]
fn catalog_rejects_an_unsafe_parent_without_chmod_or_write() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("unsafe");
    fs::create_dir(&parent).unwrap();
    set_mode(&parent, UNSAFE_DIRECTORY_MODE);

    assert!(Catalog::open(&parent.join("catalog.sqlite")).is_err());
    assert_eq!(mode(&parent), UNSAFE_DIRECTORY_MODE);
    assert!(fs::read_dir(&parent).unwrap().next().is_none());
}

fn set_mode(path: &std::path::Path, value: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(value)).unwrap();
}

fn mode(path: &std::path::Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & PERMISSION_MODE_MASK
}
