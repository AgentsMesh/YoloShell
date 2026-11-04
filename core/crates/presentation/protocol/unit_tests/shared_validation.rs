use super::{fixture::*, *};
use uuid::Uuid;
use yolo_presentation_layout::{MAX_LAYOUT_DEPTH, Tree};

#[test]
fn tab_titles_identities_and_trees_are_bounded() {
    let valid = tab();
    for title in [
        String::new(),
        INVALID_TITLE.into(),
        "trailing ".into(),
        "line\nbreak".into(),
        "x".repeat(MAX_TITLE_BYTES + 1),
    ] {
        assert_eq!(
            SharedTab::new(valid.id, title, valid.root.clone()),
            Err(ProtocolError::InvalidTitle)
        );
    }
    assert_eq!(
        SharedTab::new(Uuid::nil(), TITLE.into(), valid.root.clone()),
        Err(ProtocolError::InvalidId)
    );
    let mut root = valid.root.clone();
    for _ in 0..=MAX_LAYOUT_DEPTH {
        root = Tree::Split {
            axis: yolo_presentation_layout::Axis::Vertical,
            ratio: Default::default(),
            first: Box::new(root),
            second: Box::new(tab().root),
        };
    }
    assert_eq!(
        SharedTab::new(valid.id, TITLE.into(), root),
        Err(ProtocolError::InvalidTree)
    );
}

#[test]
fn documents_reject_duplicate_nodes_even_across_separate_tabs() {
    let mut request = update();
    let first = request.tabs[0].clone();
    request.tabs.push(first.clone());
    assert_eq!(request.validate(), Err(ProtocolError::DuplicateId));
    request.tabs[1].id = Uuid::new_v4();
    assert_eq!(request.validate(), Err(ProtocolError::DuplicateId));
    request.tabs = vec![first.clone()];
    request.tabs[0].id = first.root.panes()[0].id;
    assert_eq!(request.validate(), Err(ProtocolError::DuplicateId));
    request.tabs = vec![first; MAX_TABS + 1];
    assert_eq!(request.validate(), Err(ProtocolError::Limit));
    request.tabs = (0..=(MAX_DOCUMENT_PANES / yolo_presentation_layout::MAX_PANES))
        .map(|_| SharedTab::new(Uuid::new_v4(), TITLE.into(), tree(MAX_PANE_TREE_DEPTH)).unwrap())
        .collect();
    assert_eq!(request.validate(), Err(ProtocolError::Limit));
}

#[test]
fn request_and_document_validate_all_directly_constructed_identities() {
    for layout in [true, false] {
        let mut request = update();
        if layout {
            request.address.layout_id = Uuid::nil();
        } else {
            request.request_id = Uuid::nil();
        }
        assert_eq!(request.validate(), Err(ProtocolError::InvalidId));
    }
    let mut document = update().apply(None).unwrap();
    document.last_request_id = Uuid::nil();
    assert_eq!(document.validate(), Err(ProtocolError::InvalidId));
}
