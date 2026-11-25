use super::constants::*;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_agent_control_journal::{
    Journal, JournalAnchor, JournalError, JournalScope, Reservation, ReservationResult,
};
use yolo_crypto::Sha256Digest;
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, namespace_lease::NamespaceLease,
};

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub directory: PathBuf,
    pub scope: JournalScope,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().canonicalize().unwrap().join(DIRECTORY);
        PrivateDirectory::ensure(&directory).unwrap();
        Self {
            root,
            directory,
            scope: JournalScope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap(),
        }
    }
    pub fn retained(&self) -> PrivateDirectory {
        PrivateDirectory::open(&self.directory).unwrap()
    }
    pub fn create(&self) -> Journal {
        Journal::create_new(self.retained(), self.scope).unwrap()
    }
    pub fn reopen(&self, anchor: JournalAnchor) -> Result<Journal, JournalError> {
        Journal::open_existing(self.retained(), self.scope, anchor)
    }
    pub fn namespace(&self) -> NamespaceLease {
        NamespaceLease::capture(&[(NAMESPACE, &self.directory)]).unwrap()
    }
    pub fn read(&self, path: &Path) -> serde_json::Value {
        self.namespace().read_json(path).unwrap()
    }
    pub fn write(&self, path: &Path, value: &impl serde::Serialize) {
        self.namespace().write_json(path, value).unwrap();
    }
    pub fn version(&self, path: &Path) -> FileIdentity {
        self.namespace()
            .read_json_with_identity::<serde_json::Value>(path)
            .unwrap()
            .1
    }
}
pub fn digest() -> Sha256Digest {
    Sha256Digest::capture(&ACTION).unwrap()
}
pub fn other_digest() -> Sha256Digest {
    Sha256Digest::capture(&OTHER_ACTION).unwrap()
}
pub fn reserve(journal: &mut Journal, request: Uuid) -> Reservation {
    match journal.reserve(request, digest()).unwrap() {
        ReservationResult::New(reservation) => reservation,
        ReservationResult::Existing(_) => panic!("initial request unexpectedly exists"),
    }
}
