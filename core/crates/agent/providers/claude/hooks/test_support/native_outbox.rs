use std::os::unix::fs::PermissionsExt;

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;

pub(super) fn private_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(
        directory.path(),
        std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    directory
}
