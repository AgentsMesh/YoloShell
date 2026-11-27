use super::*;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_paths::directory::PrivateDirectory;
use yolo_presentation_layout::Layout;
use yolo_presentation_protocol::{LayoutAddress, LayoutUpdate, SharedTab};

pub const DIRECTORY: &str = "layout-store";
pub const RETIRED: &str = "retired-layout-store";
pub const REPLACEMENT: &str = "replacement.json";
pub const INVALID_BYTES: &[u8] = b"foreign layout state";
pub const TITLE: &str = "Coding";
pub const NEXT_TITLE: &str = "Review";

pub fn fixture() -> (tempfile::TempDir, PathBuf, HubId, LayoutStore) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join(DIRECTORY);
    let hub = HubId::new();
    let store = LayoutStore::open(PrivateDirectory::ensure(&path).unwrap(), hub).unwrap();
    (root, path, hub, store)
}
pub fn update(hub_id: HubId) -> LayoutUpdate {
    LayoutUpdate {
        address: LayoutAddress {
            hub_id,
            layout_id: Uuid::new_v4(),
        },
        request_id: Uuid::new_v4(),
        expected_revision: 0,
        tabs: vec![
            SharedTab::new(
                Uuid::new_v4(),
                TITLE.into(),
                Layout::new(Some(TerminalAddress {
                    hub_id,
                    terminal_id: TerminalId::new(),
                }))
                .document()
                .root
                .clone(),
            )
            .unwrap(),
        ],
    }
}
