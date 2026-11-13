use crate::GateSnapshot;
use std::fs::{self, File, Permissions};
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use yolo_release_filesystem::TreeManifest;

const SOURCE_NAME: &str = "sealed-input.rs";
const SOURCE_BYTES: &[u8] = b"pub const SEALED: bool = true;\n";
const NO_ACCESS_MODE: u32 = 0o000;
const DIRECTORY_OPEN_CONTEXT: &str = "duplicate manifest directory";
const FILE_OPEN_CONTEXT: &str = "open snapshot manifest entry";

#[test]
fn unreadable_retained_source_directory_cannot_become_an_empty_manifest() {
    let snapshot = GateSnapshot::fixture_with_source(&[(SOURCE_NAME, SOURCE_BYTES)]).unwrap();
    let source = snapshot.source();
    let original_mode = source.mode().unwrap();
    let result = {
        let _restore = PermissionRestore::deny(source.retained_file());
        TreeManifest::capture(source)
    };

    assert_permission_denied(result.unwrap_err(), DIRECTORY_OPEN_CONTEXT);
    assert_eq!(source.mode().unwrap(), original_mode);
    assert_eq!(
        fs::read(source.display_path().join(SOURCE_NAME)).unwrap(),
        SOURCE_BYTES
    );
    TreeManifest::capture(source).unwrap();
}

#[test]
fn unreadable_sealed_source_file_cannot_be_omitted_from_manifest() {
    let snapshot = GateSnapshot::fixture_with_source(&[(SOURCE_NAME, SOURCE_BYTES)]).unwrap();
    let source = snapshot.source();
    let path = source.display_path().join(SOURCE_NAME);
    let file = Arc::new(File::open(&path).unwrap());
    let original_mode = file.metadata().unwrap().permissions().mode();
    let result = {
        let _restore = PermissionRestore::deny(Arc::clone(&file));
        TreeManifest::capture(source)
    };

    assert_permission_denied(result.unwrap_err(), FILE_OPEN_CONTEXT);
    assert_eq!(file.metadata().unwrap().permissions().mode(), original_mode);
    assert_eq!(fs::read(path).unwrap(), SOURCE_BYTES);
    TreeManifest::capture(source).unwrap();
}

fn assert_permission_denied(error: anyhow::Error, context: &str) {
    assert_eq!(error.to_string(), context);
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        ErrorKind::PermissionDenied
    );
}

struct PermissionRestore {
    file: Arc<File>,
    original: Permissions,
}

impl PermissionRestore {
    fn deny(file: Arc<File>) -> Self {
        let restore = Self {
            original: file.metadata().unwrap().permissions(),
            file,
        };
        restore
            .file
            .set_permissions(Permissions::from_mode(NO_ACCESS_MODE))
            .unwrap();
        restore
    }
}

impl Drop for PermissionRestore {
    fn drop(&mut self) {
        let _ = self.file.set_permissions(self.original.clone());
    }
}
