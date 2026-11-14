mod concurrency;
#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/fixtures.rs"]
mod fixtures;
mod interference;
mod persistence;

fn parent() -> (tempfile::TempDir, yolo_paths::directory::PrivateDirectory) {
    let root = tempfile::tempdir().unwrap();
    let parent = yolo_paths::directory::PrivateDirectory::ensure(
        &root
            .path()
            .canonicalize()
            .unwrap()
            .join(constants::STATE_DIRECTORY),
    )
    .unwrap();
    (root, parent)
}
