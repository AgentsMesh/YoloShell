use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_hook_installation::HookInstallSpec;

pub const PRIVATE: u32 = 0o600;
pub const DIRECTORY: u32 = 0o700;
pub const PUBLIC_READ: u32 = 0o644;
pub const PUBLIC_TRAVERSE: u32 = 0o755;
pub const PERMISSION_MASK: u32 = 0o777;
pub const ORIGINAL: &[u8] = b"{\n  \"userPreference\": true\n}\n";

pub fn original(home: &Path, provider: &HookInstallSpec) -> PathBuf {
    let path = home.join(provider.file_name);
    fs::write(&path, ORIGINAL).unwrap();
    permissions(&path, PRIVATE);
    path
}

pub fn permissions(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

pub fn private_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    permissions(directory.path(), DIRECTORY);
    directory
}
