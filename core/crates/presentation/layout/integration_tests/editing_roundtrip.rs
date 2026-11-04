use crate::{editing_fixture::*, fixture::roundtrip};
use yolo_presentation_layout::*;

#[test]
fn keyboard_and_divider_edits_survive_serialization_without_rebinding_sessions() {
    let mut layout = grid();
    let original_panes = layout.panes().into_iter().cloned().collect::<Vec<_>>();
    assert!(layout.focus_direction(Direction::Right, bounds()).unwrap());
    assert!(layout.adjust_focused(Axis::Horizontal, true, STEP).unwrap());
    assert!(layout.adjust_focused(Axis::Vertical, false, STEP).unwrap());
    for divider in layout.split_geometry(bounds()).unwrap() {
        layout
            .resize_split(&divider.path, SplitRatio::try_from(ODD_RATIO).unwrap())
            .unwrap();
    }
    roundtrip(&layout);
    let mut restored: Layout =
        serde_json::from_slice(&serde_json::to_vec(&layout).unwrap()).unwrap();
    assert_eq!(restored, layout);
    assert_eq!(
        restored.panes().into_iter().cloned().collect::<Vec<_>>(),
        original_panes
    );
    assert_eq!(restored.focused(), id(TOP_RIGHT));
    assert_eq!(
        restored.split_geometry(bounds()).unwrap().len(),
        SPLIT_COUNT
    );
    assert!(restored.balance());
    assert!(restored.focus_direction(Direction::Down, bounds()).unwrap());
    assert_eq!(restored.focused(), id(BOTTOM_RIGHT));
    roundtrip(&restored);
}

#[test]
fn invalid_stale_paths_cannot_partially_resize_a_changed_tree() {
    let mut layout = grid();
    let removed = layout
        .split_geometry(bounds())
        .unwrap()
        .into_iter()
        .find(|split| split.path == vec![SplitBranch::Second])
        .unwrap();
    layout.close(id(TOP_RIGHT)).unwrap();
    let original = layout.clone();
    assert_eq!(
        layout.resize_split(&removed.path, SplitRatio::try_from(MIN_RATIO).unwrap()),
        Err(LayoutError::InvalidSplitPath)
    );
    assert_eq!(layout, original);
    roundtrip(&layout);
}
