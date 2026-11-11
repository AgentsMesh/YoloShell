use crate::paths::Paths;
use crate::paths::directory::PrivateDirectory;
use crate::paths::namespace_lease::NamespaceLease;
use crate::paths::private_file::PrivateFileEntry;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(super) const PRIVATE_FILE_MODE: u32 = 0o600;
pub(super) const WAL_SUFFIX: &str = "-wal";
pub(super) const SHM_SUFFIX: &str = "-shm";
const HOME_NAME: &str = "home";
const DISPLACED_NAME: &str = "displaced-home";
const NAMESPACE_LABEL: &str = "retained-vfs-test-home";
pub(super) const REPLACEMENT_SENTINEL: &str = "replacement-sentinel";
pub(super) const SENTINEL_BYTES: &[u8] = b"replacement must remain untouched";

pub(super) struct RetainedFixture {
    _temporary_root: tempfile::TempDir,
    pub(super) home: PathBuf,
    pub(super) displaced: PathBuf,
    pub(super) catalog: PathBuf,
    lease: NamespaceLease,
}

impl RetainedFixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join(HOME_NAME);
        let displaced = root.path().join(DISPLACED_NAME);
        PrivateDirectory::ensure(&home).unwrap();
        let lease = NamespaceLease::capture(&[(NAMESPACE_LABEL, &home)]).unwrap();
        let catalog = Paths::new(&home).catalog();
        Self {
            _temporary_root: root,
            home,
            displaced,
            catalog,
            lease,
        }
    }

    pub(super) fn entry(&self) -> PrivateFileEntry {
        self.lease.file(&self.catalog).unwrap()
    }

    pub(super) fn sidecar(&self, suffix: &str) -> PathBuf {
        append_suffix(&self.catalog, suffix)
    }

    pub(super) fn create_private(&self, path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    }

    pub(super) fn replace_home(&self) {
        std::fs::rename(&self.home, &self.displaced).unwrap();
        PrivateDirectory::ensure(&self.home).unwrap();
        std::fs::write(self.home.join(REPLACEMENT_SENTINEL), SENTINEL_BYTES).unwrap();
    }

    pub(super) fn assert_replacement_untouched(&self) {
        assert_eq!(
            std::fs::read(self.home.join(REPLACEMENT_SENTINEL)).unwrap(),
            SENTINEL_BYTES
        );
        assert_eq!(std::fs::read_dir(&self.home).unwrap().count(), 1);
    }
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
