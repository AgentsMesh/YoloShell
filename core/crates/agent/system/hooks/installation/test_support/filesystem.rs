use crate::{
    constants::*,
    directory::Directory,
    filesystem::{self, Snapshot},
};
use std::fs;
use std::os::unix::fs::PermissionsExt;

pub const ORIGINAL: &[u8] = b"{\"existing\":true}";
pub const UPDATED: &[u8] = b"{\"installed\":true}";
pub const CONCURRENT: &[u8] = b"{\"concurrent\":true}";

pub fn fixture(existing: bool) -> (tempfile::TempDir, Directory, Snapshot) {
    let home = tempfile::tempdir().unwrap();
    fs::set_permissions(
        home.path(),
        fs::Permissions::from_mode(OWNER_DIRECTORY_MODE),
    )
    .unwrap();
    let provider_home = home.path().join("provider");
    fs::create_dir(&provider_home).unwrap();
    if existing {
        fs::write(provider_home.join(CODEX_FILE), ORIGINAL).unwrap();
    }
    let directory = Directory::open(&provider_home).unwrap();
    let snapshot = filesystem::read(&directory, CODEX_FILE).unwrap();
    (home, directory, snapshot)
}
