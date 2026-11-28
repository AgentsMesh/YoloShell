use crate::{CreationJournal, JOURNAL_FILE};
use std::path::PathBuf;
use yolo_hub_identity::HubId;
use yolo_paths::directory::PrivateDirectory;

pub const JOURNAL_DIRECTORY: &str = "journal";
pub const MOVED_DIRECTORY: &str = "moved";
pub const OTHER_DIRECTORY: &str = "other";
pub const PRIVATE_MODE: u32 = 0o600;
pub const PUBLIC_MODE: u32 = 0o644;
pub const MODE_MASK: u32 = 0o777;
pub const CAPACITY_SENTINEL: u64 = 1;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub path: PathBuf,
    pub entry: HubId,
}

impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap().join(JOURNAL_DIRECTORY);
        PrivateDirectory::ensure(&path).unwrap();
        Self {
            root,
            path,
            entry: HubId::new(),
        }
    }
    pub fn open(&self) -> CreationJournal {
        CreationJournal::open(self.parent(), self.entry).unwrap()
    }
    pub fn parent(&self) -> PrivateDirectory {
        PrivateDirectory::open(&self.path).unwrap()
    }
    pub fn file(&self) -> PathBuf {
        self.path.join(JOURNAL_FILE)
    }
    pub fn stored(&self) -> serde_json::Value {
        yolo_paths::read_json(&self.file()).unwrap()
    }
    pub fn identity(&self) -> yolo_paths::identity::FileIdentity {
        let path = self.file();
        yolo_paths::identity::FileIdentity::inspect_bounded(
            &std::fs::metadata(&path).unwrap(),
            &path,
            yolo_paths::constants::MAX_DURABLE_JSON_BYTES,
        )
        .unwrap()
    }
    pub fn replace(&self, value: &serde_json::Value) {
        yolo_paths::atomic_write_json(&self.file(), value).unwrap();
    }
}
