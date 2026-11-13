use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_presentation_layout::{Axis, Layout, PaneLayoutDocument, Pane, SplitRatio, Tree};

pub fn terminal() -> TerminalAddress {
    TerminalAddress {
        hub_id: HubId::new(),
        terminal_id: TerminalId::new(),
    }
}

pub fn tree(count: usize, ids: &mut Vec<Uuid>) -> Tree {
    if count == 1 {
        let id = Uuid::new_v4();
        ids.push(id);
        return Tree::Pane(Pane {
            id,
            terminal: Some(terminal()),
        });
    }
    Tree::Split {
        axis: Axis::Horizontal,
        ratio: SplitRatio::default(),
        first: Box::new(tree(count / 2, ids)),
        second: Box::new(tree(count - count / 2, ids)),
    }
}

pub fn workspace(count: usize) -> PaneLayoutDocument {
    let mut ids = Vec::new();
    let root = tree(count, &mut ids);
    PaneLayoutDocument {
        root,
        focused: ids[0],
    }
}

pub fn roundtrip(layout: &Layout) {
    let serialized = serde_json::to_vec(layout).unwrap();
    assert_eq!(
        serde_json::from_slice::<Layout>(&serialized).unwrap(),
        *layout
    );
}
