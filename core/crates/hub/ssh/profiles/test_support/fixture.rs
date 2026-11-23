use super::constants::*;
use anyhow::Result;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_ssh_profiles::{SSH_PROFILES_DIRECTORY, SshProfileConfiguration, SshProfileStore};
use yolo_paths::directory::PrivateDirectory;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub path: PathBuf,
    pub hub: PeerIdentity,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .canonicalize()
            .unwrap()
            .join(SSH_PROFILES_DIRECTORY);
        PrivateDirectory::open_or_create_strict(&path).unwrap();
        Self {
            root,
            path,
            hub: yolo_hub_delegation_test_support::Identity::new().identity,
        }
    }
    pub fn open(&self, seed: Option<SshProfileConfiguration>) -> Result<SshProfileStore> {
        SshProfileStore::open(PrivateDirectory::open(&self.path)?, self.hub, seed)
    }
    pub fn record(&self) -> PathBuf {
        self.path.join(RECORD_FILE)
    }
    pub fn bytes(&self) -> Vec<u8> {
        fs::read(self.record()).unwrap()
    }
    pub fn sibling(&self, name: &str) -> PathBuf {
        self.path.parent().unwrap().join(name)
    }
}
pub fn write_new(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}
