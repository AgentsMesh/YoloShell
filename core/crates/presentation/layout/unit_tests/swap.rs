use crate::{editing_fixture::*, fixture::roundtrip};
use std::collections::BTreeMap;
use yolo_presentation_layout::{LayoutError, SplitBranch, SplitRatio, SwapDirection};

#[test]
fn swapping_moves_complete_panes_and_preserves_split_topology_ratios_and_focus() {
    let mut layout = grid();
    layout
        .resize_split(&[], SplitRatio::try_from(ODD_RATIO).unwrap())
        .unwrap();
    layout
        .resize_split(
            &[SplitBranch::First],
            SplitRatio::try_from(STEP + MIN_RATIO_VALUE).unwrap(),
        )
        .unwrap();
    let shape = layout.split_geometry(bounds()).unwrap();
    let bindings = layout
        .panes()
        .into_iter()
        .map(|pane| (pane.id, pane.terminal))
        .collect::<BTreeMap<_, _>>();
    let before = layout.geometry(bounds()).unwrap();
    assert!(layout.swap_panes(id(TOP_LEFT), id(BOTTOM_RIGHT)).unwrap());
    assert_eq!(layout.focused(), id(TOP_LEFT));
    assert_eq!(layout.split_geometry(bounds()).unwrap(), shape);
    assert_eq!(
        layout
            .panes()
            .into_iter()
            .map(|pane| (pane.id, pane.terminal))
            .collect::<BTreeMap<_, _>>(),
        bindings
    );
    let after = layout.geometry(bounds()).unwrap();
    assert_eq!(
        after
            .iter()
            .find(|pane| pane.pane == id(TOP_LEFT))
            .unwrap()
            .rect,
        before
            .iter()
            .find(|pane| pane.pane == id(BOTTOM_RIGHT))
            .unwrap()
            .rect
    );
    assert_eq!(
        after
            .iter()
            .find(|pane| pane.pane == id(BOTTOM_RIGHT))
            .unwrap()
            .rect,
        before
            .iter()
            .find(|pane| pane.pane == id(TOP_LEFT))
            .unwrap()
            .rect
    );
    roundtrip(&layout);
}

#[test]
fn previous_and_next_swap_wrap_in_leaf_order_without_changing_focused_identity() {
    let mut layout = grid();
    let original = layout.clone();
    assert!(layout.swap_focused(SwapDirection::Previous).unwrap());
    assert_eq!(layout.panes().last().unwrap().id, original.focused());
    assert_eq!(layout.focused(), original.focused());
    assert!(layout.swap_focused(SwapDirection::Next).unwrap());
    assert_eq!(layout, original);
    assert!(layout.swap_focused(SwapDirection::Next).unwrap());
    assert_eq!(layout.panes()[PATH_STEP].id, original.focused());
    assert!(layout.swap_focused(SwapDirection::Previous).unwrap());
    assert_eq!(layout, original);
}

#[test]
fn invalid_or_identical_swap_targets_never_partially_change_layout() {
    let mut layout = grid();
    let original = layout.clone();
    assert!(
        !layout
            .swap_panes(layout.focused(), layout.focused())
            .unwrap()
    );
    for (first, second) in [
        (uuid::Uuid::new_v4(), layout.focused()),
        (layout.focused(), uuid::Uuid::new_v4()),
    ] {
        assert_eq!(
            layout.swap_panes(first, second),
            Err(LayoutError::UnknownPane)
        );
        assert_eq!(layout, original);
    }
    let mut single = yolo_presentation_layout::Layout::default();
    let original = single.clone();
    for direction in [SwapDirection::Next, SwapDirection::Previous] {
        assert!(!single.swap_focused(direction).unwrap());
        assert_eq!(single, original);
    }
}

const MIN_RATIO_VALUE: u16 = yolo_presentation_layout::MIN_RATIO;
