use crate::PinnedArtifact;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::{NO_FILE_FLAGS, PRIVATE_DIRECTORY_MODE};

const ARCHIVE_BYTES: &[u8] = b"artifact custody witness";
const SOURCE_NAME: &str = "source.zip";
const MOVED_NAME: &str = "retained-original";
const SENTINEL_NAME: &str = "foreign-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve unrelated replacement";
const READ_ONLY_MODE: u32 = 0o400;
const MODE_MASK: u32 = 0o777;

#[test]
fn artifact_drop_preserves_replacement_after_integrity_rejection() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join(SOURCE_NAME);
    fs::write(&source, ARCHIVE_BYTES).unwrap();
    let artifact = PinnedArtifact::from_opened(&File::open(source).unwrap()).unwrap();
    let path = rebind(&artifact, &fixture.path().join(MOVED_NAME));
    assert!(artifact.sha256().is_err());
    drop(artifact);
    assert_preserved_and_remove(&path);
}

#[test]
fn failed_artifact_capture_preserves_the_replacement_namespace() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join(SOURCE_NAME);
    fs::write(&source, ARCHIVE_BYTES).unwrap();
    let mut private = None;
    let result = PinnedArtifact::from_opened_with(&File::open(source).unwrap(), |artifact| {
        private = Some(rebind(artifact, &fixture.path().join(MOVED_NAME)));
        Ok(())
    });
    assert!(result.is_err());
    assert_preserved_and_remove(&private.unwrap());
}

fn rebind(artifact: &PinnedArtifact, moved: &Path) -> PathBuf {
    let path = artifact.root.display_path().to_owned();
    artifact.root.set_flags(NO_FILE_FLAGS).unwrap();
    artifact.root.set_mode(PRIVATE_DIRECTORY_MODE).unwrap();
    fs::rename(&path, moved).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join(SENTINEL_NAME), SENTINEL_BYTES).unwrap();
    fs::set_permissions(
        path.join(SENTINEL_NAME),
        fs::Permissions::from_mode(READ_ONLY_MODE),
    )
    .unwrap();
    path
}

fn assert_preserved_and_remove(path: &Path) {
    let sentinel = path.join(SENTINEL_NAME);
    assert_eq!(fs::read(&sentinel).unwrap(), SENTINEL_BYTES);
    assert_eq!(
        fs::metadata(&sentinel).unwrap().permissions().mode() & MODE_MASK,
        READ_ONLY_MODE
    );
    fs::remove_file(sentinel).unwrap();
    fs::remove_dir(path).unwrap();
}
