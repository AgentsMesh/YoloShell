use super::{constants::*, model};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use yolo_hub_client_access_protocol::AccessConfiguration;
use yolo_hub_client_access_store::{CLIENT_ACCESS_DIRECTORY, ClientAccessStore};
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, namespace_lease::NamespaceLease,
};

pub struct Fixture {
    pub path: PathBuf,
    pub root: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .canonicalize()
            .unwrap()
            .join(CLIENT_ACCESS_DIRECTORY);
        PrivateDirectory::open_or_create_strict(&path).unwrap();
        Self { path, root }
    }

    pub fn directory(&self) -> PrivateDirectory {
        PrivateDirectory::open(&self.path).unwrap()
    }

    pub fn open(&self, seed: Option<AccessConfiguration>) -> anyhow::Result<ClientAccessStore> {
        ClientAccessStore::open(self.directory(), model::hub(), seed)
    }

    pub fn record(&self) -> PathBuf {
        self.path.join(RECORD_FILE)
    }

    pub fn lock(&self) -> PathBuf {
        self.path.join(LOCK_FILE)
    }

    pub fn bytes(&self) -> Vec<u8> {
        fs::read(self.record()).unwrap()
    }

    pub fn file_identity(&self) -> FileIdentity {
        NamespaceLease::from_directories(vec![(TEST_NAMESPACE, self.directory())])
            .unwrap()
            .read_json_with_identity::<serde_json::Value>(&self.record())
            .unwrap()
            .1
    }

    pub fn sibling(&self, name: &str) -> PathBuf {
        self.root.path().canonicalize().unwrap().join(name)
    }
}

pub fn write_new(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

pub fn assert_poisoned(store: &mut ClientAccessStore, original: &AccessConfiguration) {
    assert!(store.current().is_err());
    let mut candidate = original.clone();
    candidate.revision = original.revision.saturating_add(REVISION_STEP);
    assert!(store.replace(original.revision, candidate).is_err());
    assert!(store.current().is_err());
}
