use std::path::PathBuf;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_presentation_layout::{Axis, Layout};

mod constants;
mod creation;
pub use constants::*;
pub use creation::intent;
pub fn fixture() -> (tempfile::TempDir, PathBuf, HubId) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join(DIRECTORY);
    (root, path, HubId::new())
}
pub fn layout() -> Layout {
    let mut layout = Layout::new(Some(TerminalAddress {
        hub_id: HubId::new(),
        terminal_id: TerminalId::new(),
    }));
    let second = layout.split(Axis::Horizontal, None).unwrap();
    layout.split(Axis::Vertical, None).unwrap();
    layout.focus(second).unwrap();
    layout
}
