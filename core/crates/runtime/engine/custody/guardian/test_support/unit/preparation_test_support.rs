use crate::paths::identity::FileIdentity;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const PRIVATE_FILE_MODE: u32 = 0o600;
const IDENTITY_FILE_MAX_BYTES: u64 = 64;
const IDENTITY_FILE_CONTENT: &[u8] = b"guardian-preparation";

pub(super) fn distinct_file_identities() -> (FileIdentity, FileIdentity) {
    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("original");
    let replacement = directory.path().join("replacement");
    for path in [&original, &replacement] {
        std::fs::write(path, IDENTITY_FILE_CONTENT).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    }
    (file_identity(&original), file_identity(&replacement))
}

fn file_identity(path: &Path) -> FileIdentity {
    FileIdentity::inspect_bounded(&path.metadata().unwrap(), path, IDENTITY_FILE_MAX_BYTES).unwrap()
}
