use crate::fixture::*;
use uuid::Uuid;
use yolo_presentation_layout::*;

#[test]
fn layout_operations_preserve_bound_sessions_and_close_only_the_selected_pane() {
    let address = terminal();
    let mut layout = Layout::new(Some(address));
    let first = layout.focused();
    assert_eq!(layout.panes()[0].terminal, Some(address));
    let second_address = terminal();
    let second = layout
        .split(Axis::Horizontal, Some(second_address))
        .unwrap();
    let third = layout.split(Axis::Vertical, None).unwrap();
    assert_eq!(layout.focused(), third);
    layout.bind(third, Some(address)).unwrap();
    layout
        .resize_parent(first, SplitRatio::try_from(300).unwrap())
        .unwrap();
    layout
        .resize_parent(third, SplitRatio::try_from(700).unwrap())
        .unwrap();
    assert_eq!(
        layout.panes().iter().map(|p| p.id).collect::<Vec<_>>(),
        [first, second, third]
    );
    roundtrip(&layout);
    layout.close(second).unwrap();
    assert_eq!(layout.focused(), third);
    assert_eq!(
        layout
            .panes()
            .iter()
            .map(|p| p.terminal)
            .collect::<Vec<_>>(),
        [Some(address), Some(address)]
    );
    layout.focus(first).unwrap();
    layout.close(first).unwrap();
    assert_eq!(layout.focused(), third);
    assert!(matches!(layout.document().root, Tree::Pane(_)));
    layout.bind(third, None).unwrap();
    roundtrip(&layout);
    assert_eq!(layout.panes()[0].terminal, None);
}

#[test]
fn every_failed_edit_keeps_the_prior_document_unchanged() {
    let mut layout = Layout::default();
    let original = layout.clone();
    let missing = Uuid::new_v4();
    assert_eq!(layout.focus(missing), Err(LayoutError::UnknownPane));
    assert_eq!(
        layout.bind(missing, Some(terminal())),
        Err(LayoutError::UnknownPane)
    );
    assert_eq!(layout.close(missing), Err(LayoutError::UnknownPane));
    assert_eq!(layout.close(layout.focused()), Err(LayoutError::LastPane));
    assert_eq!(
        layout.resize_parent(missing, SplitRatio::default()),
        Err(LayoutError::UnknownPane)
    );
    assert_eq!(layout, original);
    layout.split(Axis::Horizontal, None).unwrap();
    let original = layout.clone();
    assert_eq!(
        layout.resize_parent(missing, SplitRatio::default()),
        Err(LayoutError::UnknownPane)
    );
    assert_eq!(layout, original);
}

#[test]
fn splitting_both_subtrees_and_closing_nested_first_children_preserves_focus() {
    let mut layout = Layout::default();
    let first = layout.focused();
    let second = layout.split(Axis::Horizontal, None).unwrap();
    layout.focus(first).unwrap();
    let third = layout.split(Axis::Vertical, None).unwrap();
    layout.resize_parent(third, SplitRatio::default()).unwrap();
    layout.bind(first, Some(terminal())).unwrap();
    layout.close(first).unwrap();
    layout.close(third).unwrap();
    assert_eq!(layout.focused(), second);
    roundtrip(&layout);
}

#[test]
fn pane_count_and_split_depth_are_bounded_without_partially_installing_edits() {
    let mut deep = Layout::default();
    for _ in 0..MAX_LAYOUT_DEPTH {
        deep.split(Axis::Vertical, None).unwrap();
    }
    let original = deep.clone();
    assert_eq!(deep.split(Axis::Horizontal, None), Err(LayoutError::Limit));
    assert_eq!(deep, original);
    let mut wide = Layout::try_from(workspace(MAX_PANES)).unwrap();
    let original = wide.clone();
    assert_eq!(wide.split(Axis::Horizontal, None), Err(LayoutError::Limit));
    assert_eq!(wide, original);
    roundtrip(&wide);
}
