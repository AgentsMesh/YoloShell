use std::{path::PathBuf, time::Duration};
use yolo_hub_identity::HubId;
use yolo_paths::private_file::PrivateFileEntry;
use yolo_tui_tab::TabStore;
use yolo_tui_client_control_protocol::{
    CONTROL_DESCRIPTOR_NAME, ControlDescriptor, MAX_CONTROL_FRAME_BYTES,
};

pub const TEST_DEADLINE: Duration = Duration::from_secs(5);
pub const DIRECTORY_NAME: &str = "client-state";
pub const SAVED_ENTRY: &str = "saved-entry";
pub const REPLACEMENT_CONTENT: &[u8] = b"replacement must survive";
pub const PRIVATE_MODE: u32 = 0o600;
pub const PUBLIC_MODE: u32 = 0o644;
pub const LONG_PATH_REPETITIONS: usize = 16;
pub const OVERFLOW_BYTE: usize = 1;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub path: PathBuf,
    pub hub: HubId,
}
impl Fixture {
    pub fn new(long: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let suffix = if long {
            DIRECTORY_NAME.repeat(LONG_PATH_REPETITIONS)
        } else {
            DIRECTORY_NAME.into()
        };
        let path = root.path().join(suffix);
        let hub = HubId::new();
        Self { root, path, hub }
    }
    pub fn store(&self) -> TabStore {
        TabStore::open(&self.path, self.hub).unwrap()
    }
    pub fn descriptor_path(&self) -> PathBuf {
        self.path.join(CONTROL_DESCRIPTOR_NAME)
    }
    pub fn descriptor(&self) -> ControlDescriptor {
        yolo_paths::read_json(&self.descriptor_path()).unwrap()
    }
    pub fn socket(&self) -> PathBuf {
        yolo_paths::Paths::new(&self.path)
            .worker_socket(self.descriptor().endpoint)
            .unwrap()
    }
    pub fn verify_descriptor(&self) {
        PrivateFileEntry::open(&self.descriptor_path())
            .unwrap()
            .open_read(MAX_CONTROL_FRAME_BYTES as u64)
            .unwrap();
    }
}
