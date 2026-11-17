use super::*;
use std::{
    fs,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::PathBuf,
};

pub(super) const HOST: &str = "host.sock";
pub(super) const HUB: &str = "hub.sock";
pub(super) const UNKNOWN: &str = "preserve-me";
pub(super) const STAGE: &str = ".b-00000000000000000000000000000000";
pub(super) const CONTENT: &[u8] = b"unrelated durable data";
pub(super) const WRONG_MODE: u32 = 0o644;
const TEMP_ROOT: &str = "/tmp";
const TEMP_PREFIX: &str = "u-";

pub(super) struct Fixture {
    pub _temporary: tempfile::TempDir,
    pub root: PrivateDirectory,
    pub location: SocketNamespaceLocation,
    pub directory: PrivateDirectory,
}
impl Fixture {
    pub fn new() -> Self {
        let temporary = tempfile::Builder::new()
            .prefix(TEMP_PREFIX)
            .tempdir_in(TEMP_ROOT)
            .unwrap();
        let base = temporary.path().canonicalize().unwrap();
        let mut location = super::super::test_construction::unique_location(&base);
        let name = location.directory.file_name().unwrap().to_owned();
        location.root = base;
        location.directory = location.root.join(name);
        location.overridden_root = true;
        let root = PrivateDirectory::ensure(&location.root).unwrap();
        let directory = root.ensure_descendant(&location.directory).unwrap();
        Self {
            _temporary: temporary,
            root,
            location,
            directory,
        }
    }
    pub fn listener(&self, name: &str) -> UnixListener {
        let path = self.location.directory.join(name);
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
        )
        .unwrap();
        listener
    }
    pub fn recover(&self) -> Result<()> {
        let mut lifecycle = SocketNamespaceLifecycleLock::acquire(&self.root)?;
        recover(
            &self.root,
            &self.location,
            &self.directory,
            &mut lifecycle,
            || Ok(()),
        )
    }
    pub fn quarantines(&self) -> Vec<PathBuf> {
        fs::read_dir(self.root.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(QUARANTINE_PREFIX)
            })
            .collect()
    }
}
