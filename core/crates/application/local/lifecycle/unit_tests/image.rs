use std::{fs, os::unix::fs::PermissionsExt};
use tempfile::tempdir;
use yolo_local_lifecycle::image;

#[test]
fn image_hashes_a_regular_executable_and_rejects_a_selector_link() {
    let directory = tempdir().unwrap();
    let executable = directory.path().join("yolo");
    fs::write(&executable, b"candidate").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let observed = image(&executable).unwrap();
    assert_eq!(observed.size_bytes(), 9);
    assert_eq!(observed.sha256().len(), 64);

    let link = directory.path().join("yolo-link");
    std::os::unix::fs::symlink(&executable, &link).unwrap();
    assert!(image(&link).is_err());
    assert_eq!(image(&fs::canonicalize(&link).unwrap()).unwrap(), observed);
}

#[test]
fn image_rejects_empty_files() {
    let directory = tempdir().unwrap();
    let executable = directory.path().join("yolo");
    fs::write(&executable, b"").unwrap();
    assert!(image(&executable).is_err());
}
