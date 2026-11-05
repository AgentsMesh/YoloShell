use super::*;
use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_presentation_layout::{Axis, Layout, SplitRatio, Tree};
#[path = "constants.rs"]
mod constants;
pub use constants::*;

pub fn tab() -> SharedTab {
    SharedTab::new(
        Uuid::new_v4(),
        TITLE.into(),
        Layout::new(Some(TerminalAddress {
            hub_id: HubId::new(),
            terminal_id: TerminalId::new(),
        }))
        .document()
        .root
        .clone(),
    )
    .unwrap()
}
pub fn update() -> LayoutUpdate {
    LayoutUpdate {
        address: LayoutAddress {
            hub_id: HubId::new(),
            layout_id: Uuid::new_v4(),
        },
        request_id: Uuid::new_v4(),
        expected_revision: 0,
        tabs: vec![tab()],
    }
}
pub fn tree(depth: usize) -> Tree {
    if depth == 0 {
        return Layout::default().document().root.clone();
    }
    Tree::Split {
        axis: Axis::Horizontal,
        ratio: SplitRatio::default(),
        first: Box::new(tree(depth - 1)),
        second: Box::new(tree(depth - 1)),
    }
}
