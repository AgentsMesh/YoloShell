use crate::GateSnapshot;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use yolo_release_contracts::SOURCE_DIRECTORY_NAME;

const SENTINEL: &str = "foreign-sentinel";
const FOREIGN_BYTES: &[u8] = b"unrelated retained data";
const READ_ONLY_MODE: u32 = 0o400;
const MODE_MASK: u32 = 0o777;
const SOURCE_NAME: &str = "source.rs";
const SOURCE_BYTES: &[u8] = b"pub const VALUE: bool = true;\n";

#[test]
fn normal_snapshot_close_removes_original_read_only_tree_and_external_symlinks_only() {
    let snapshot = GateSnapshot::fixture_with_source(&[(SOURCE_NAME, SOURCE_BYTES)]).unwrap();
    let root = snapshot.root_path();
    let foreign = tempfile::tempdir().unwrap();
    let sentinel = foreign.path().join(SENTINEL);
    fs::write(&sentinel, FOREIGN_BYTES).unwrap();
    symlink(
        foreign.path(),
        snapshot.output().display_path().join("external"),
    )
    .unwrap();
    snapshot.close().unwrap();
    assert!(!root.exists());
    assert_eq!(fs::read(sentinel).unwrap(), FOREIGN_BYTES);
}

#[test]
fn root_replacement_survives_snapshot_drop_with_unchanged_bytes_and_modes() {
    let snapshot = GateSnapshot::fixture_with_source(&[(SOURCE_NAME, SOURCE_BYTES)]).unwrap();
    let root = snapshot.root_path();
    let saved = tempfile::tempdir().unwrap();
    let original = saved.path().join("original");
    fs::rename(&root, &original).unwrap();
    fs::create_dir(&root).unwrap();
    let sentinel = replacement_sentinel(&root);
    drop(snapshot);
    assert_replacement(&sentinel);
    assert_eq!(
        fs::read(original.join(SOURCE_DIRECTORY_NAME).join(SOURCE_NAME)).unwrap(),
        SOURCE_BYTES
    );
    fs::remove_file(sentinel).unwrap();
    fs::remove_dir(root).unwrap();
    yolo_release_filesystem::restore_owner_write(&original).unwrap();
}

#[test]
fn source_replacement_is_preserved_by_explicit_close_and_drop() {
    let snapshot = GateSnapshot::fixture_with_source(&[(SOURCE_NAME, SOURCE_BYTES)]).unwrap();
    let root = snapshot.root_path();
    let source = snapshot.source().display_path().to_owned();
    let moved = root.join("original-source");
    fs::rename(&source, &moved).unwrap();
    fs::create_dir(&source).unwrap();
    let sentinel = replacement_sentinel(&source);
    let error = snapshot.close().unwrap_err();
    assert!(format!("{error:#}").contains("snapshot child directory identity changed"));
    assert_replacement(&sentinel);
    assert_eq!(fs::read(moved.join(SOURCE_NAME)).unwrap(), SOURCE_BYTES);
    yolo_release_filesystem::restore_owner_write(&root).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unconfirmed_external_custody_preserves_snapshot_but_confirmed_custody_releases_it() {
    let snapshot = GateSnapshot::fixture_with_source(&[(SOURCE_NAME, SOURCE_BYTES)]).unwrap();
    let root = snapshot.root_path();
    drop(
        snapshot
            .source()
            .retain_resource_use("container exact identity pending")
            .unwrap(),
    );
    let error = snapshot.close().unwrap_err();
    assert!(format!("{error:#}").contains("container exact identity pending"));
    assert_eq!(
        fs::read(root.join(SOURCE_DIRECTORY_NAME).join(SOURCE_NAME)).unwrap(),
        SOURCE_BYTES
    );
    yolo_release_filesystem::restore_owner_write(&root).unwrap();
    fs::remove_dir_all(root).unwrap();

    let snapshot = GateSnapshot::fixture().unwrap();
    let root = snapshot.root_path();
    snapshot
        .source()
        .retain_resource_use("container confirmed absent")
        .unwrap()
        .confirm_released();
    snapshot.close().unwrap();
    assert!(!root.exists());
}

fn replacement_sentinel(directory: &std::path::Path) -> std::path::PathBuf {
    let sentinel = directory.join(SENTINEL);
    fs::write(&sentinel, FOREIGN_BYTES).unwrap();
    fs::set_permissions(&sentinel, fs::Permissions::from_mode(READ_ONLY_MODE)).unwrap();
    sentinel
}

fn assert_replacement(sentinel: &std::path::Path) {
    assert_eq!(fs::read(sentinel).unwrap(), FOREIGN_BYTES);
    assert_eq!(
        fs::metadata(sentinel).unwrap().permissions().mode() & MODE_MASK,
        READ_ONLY_MODE
    );
}
