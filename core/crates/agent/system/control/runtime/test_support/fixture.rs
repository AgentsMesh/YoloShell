use super::{constants::*, records};
use std::path::{Path, PathBuf};
use yolo_agent_control_runtime::{
    ANCHOR_EXTENSION, ANCHOR_PREFIX, ControlRuntime, FreshInitialization, JOURNAL_DIRECTORY,
    JournalError,
};
use yolo_crypto::Sha256Digest;
use yolo_paths::{Paths, directory::PrivateDirectory, namespace_lease::NamespaceLease};
use yolo_terminal_model::{BootstrapDelivery, RuntimeRecord};

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub paths: Paths,
    pub runtime: RuntimeRecord,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::new(root.path().canonicalize().unwrap());
        paths.ensure_base().unwrap();
        let launch = records::launch(&paths);
        let durable = records::runtime(&launch);
        let directory = PrivateDirectory::ensure(&paths.session_dir(durable.terminal_id)).unwrap();
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)]).unwrap();
        for (path, record) in [
            (
                paths.runtime_record(durable.terminal_id),
                serde_json::to_value(&durable).unwrap(),
            ),
            (
                paths.worker_launch(durable.terminal_id),
                serde_json::to_value(&launch).unwrap(),
            ),
            (
                paths.manifest(durable.terminal_id),
                serde_json::to_value(records::manifest(&durable)).unwrap(),
            ),
            (
                paths.worker_startup_checkpoint(durable.terminal_id),
                serde_json::to_value(records::checkpoint(&launch, &durable)).unwrap(),
            ),
        ] {
            namespace.write_json(&path, &record).unwrap();
        }
        let mut runtime = durable;
        runtime.bootstrap_delivery = BootstrapDelivery::NotRequired;
        Self {
            root,
            paths,
            runtime,
        }
    }
    pub fn session(&self) -> PrivateDirectory {
        PrivateDirectory::open(&self.paths.session_dir(self.runtime.terminal_id)).unwrap()
    }
    pub fn namespace(&self) -> NamespaceLease {
        NamespaceLease::from_directories(vec![(NAMESPACE, self.session())]).unwrap()
    }
    pub fn proof(&self) -> Result<FreshInitialization, JournalError> {
        FreshInitialization::capture(self.session(), &self.paths, &self.runtime)
    }
    pub fn initialize(&self) -> ControlRuntime {
        ControlRuntime::initialize_fresh(self.proof().unwrap()).unwrap()
    }
    pub fn reopen(&self, runtime: &RuntimeRecord) -> Result<ControlRuntime, JournalError> {
        ControlRuntime::open_existing(self.session(), &self.paths, runtime)
    }
    pub fn anchor(&self) -> PathBuf {
        self.paths
            .session_dir(self.runtime.terminal_id)
            .join(format!("{ANCHOR_PREFIX}{}", self.runtime.runtime_epoch))
            .with_extension(ANCHOR_EXTENSION)
    }
    pub fn journal(&self) -> PathBuf {
        self.paths
            .session_dir(self.runtime.terminal_id)
            .join(JOURNAL_DIRECTORY)
            .join(self.runtime.runtime_epoch.to_string())
    }
    pub fn read<T: serde::de::DeserializeOwned>(&self, path: &Path) -> T {
        self.namespace().read_json(path).unwrap()
    }
    pub fn write(&self, path: &Path, value: &impl serde::Serialize) {
        self.namespace().write_json(path, value).unwrap();
    }
}
pub fn digest() -> Sha256Digest {
    Sha256Digest::capture(&ACTION).unwrap()
}
pub fn other_digest() -> Sha256Digest {
    Sha256Digest::capture(&OTHER_ACTION).unwrap()
}
pub fn reserve(
    owner: &mut ControlRuntime,
    runtime: &RuntimeRecord,
    request: uuid::Uuid,
) -> yolo_agent_control_runtime::Reservation {
    match owner.reserve(runtime, request, digest()).unwrap() {
        yolo_agent_control_runtime::ReservationResult::New(reservation) => reservation,
        yolo_agent_control_runtime::ReservationResult::Existing(_) => {
            panic!("{EXPECTED_NEW_RESERVATION}")
        }
    }
}
