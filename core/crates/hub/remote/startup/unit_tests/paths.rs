use crate::not_in_releases;
use yolo_paths::directory::PrivateDirectory;
#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;

#[test]
fn mutable_paths_cannot_use_immutable_release_ancestors_or_aliases() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join(ROOT_NAME);
    let releases = PrivateDirectory::open_or_create_strict(
        &root.join(yolo_distribution_bundle::RELEASES_DIRECTORY),
    )
    .unwrap();
    assert!(not_in_releases(&root.join(STATE_NAME), &root).is_ok());
    assert!(not_in_releases(releases.path(), &root).is_err());
    assert!(not_in_releases(&releases.path().join(MISSING_CHILD).join(STATE_NAME), &root).is_err());
    let alias = root.join(UPPER_RELEASES);
    if alias.exists() {
        assert!(not_in_releases(&alias.join(MISSING_CHILD).join(STATE_NAME), &root).is_err());
    }
    let alias = root.join(SYMLINK_NAME);
    std::os::unix::fs::symlink(releases.path(), &alias).unwrap();
    assert!(not_in_releases(&alias.join(MISSING_CHILD).join(STATE_NAME), &root).is_err());
    let relative = std::path::Path::new(STATE_NAME);
    assert!(not_in_releases(relative, &root).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn mutable_paths_cannot_bypass_the_guard_through_the_data_volume_firmlink() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join(ROOT_NAME);
    let releases = PrivateDirectory::open_or_create_strict(
        &root.join(yolo_distribution_bundle::RELEASES_DIRECTORY),
    )
    .unwrap();
    let alias = std::path::Path::new(SYSTEM_VOLUME_PREFIX).join(
        releases
            .path()
            .strip_prefix(std::path::MAIN_SEPARATOR_STR)
            .unwrap(),
    );
    assert!(
        alias.exists(),
        "the native macOS temporary directory must expose its data-volume alias"
    );
    assert!(not_in_releases(&alias.join(STATE_NAME), &root).is_err());
}
