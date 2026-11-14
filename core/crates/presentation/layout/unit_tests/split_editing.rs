use crate::editing_fixture::*;
use yolo_presentation_layout::*;

#[test]
fn adjustments_follow_the_focused_side_of_the_nearest_matching_axis() {
    let balanced = u16::from(SplitRatio::default());
    for focused in [TOP_LEFT, TOP_RIGHT, BOTTOM_LEFT, BOTTOM_RIGHT] {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            for grow in [false, true] {
                let mut layout = grid();
                layout.focus(id(focused)).unwrap();
                let original_panes = layout.panes().into_iter().cloned().collect::<Vec<_>>();
                let (path, first_side) = match axis {
                    Axis::Horizontal => (vec![], matches!(focused, TOP_LEFT | BOTTOM_LEFT)),
                    Axis::Vertical => (
                        vec![if matches!(focused, TOP_LEFT | BOTTOM_LEFT) {
                            SplitBranch::First
                        } else {
                            SplitBranch::Second
                        }],
                        matches!(focused, TOP_LEFT | TOP_RIGHT),
                    ),
                };
                assert_eq!(layout.adjust_focused(axis, grow, STEP), Ok(true));
                assert_eq!(
                    ratio_at(&layout, &path),
                    if grow == first_side {
                        balanced + STEP
                    } else {
                        balanced - STEP
                    }
                );
                assert_eq!(layout.focused(), id(focused));
                assert_eq!(
                    layout.panes().into_iter().cloned().collect::<Vec<_>>(),
                    original_panes
                );
            }
        }
    }
}

#[test]
fn adjusting_clamps_without_overflow_or_falling_through_to_a_farther_ancestor() {
    let mut layout = Layout::try_from(PaneLayoutDocument {
        root: split(
            Axis::Horizontal,
            split(Axis::Horizontal, pane(TOP_LEFT), pane(TOP_RIGHT)),
            pane(BOTTOM_LEFT),
        ),
        focused: id(TOP_LEFT),
    })
    .unwrap();
    let root_ratio = ratio_at(&layout, &[]);
    assert!(
        layout
            .adjust_focused(Axis::Horizontal, true, u16::MAX)
            .unwrap()
    );
    assert_eq!(ratio_at(&layout, &[SplitBranch::First]), MAX_RATIO);
    assert_eq!(
        layout.adjust_focused(Axis::Horizontal, true, STEP),
        Ok(false)
    );
    assert_eq!(ratio_at(&layout, &[]), root_ratio);
    assert!(
        layout
            .adjust_focused(Axis::Horizontal, false, u16::MAX)
            .unwrap()
    );
    assert_eq!(ratio_at(&layout, &[SplitBranch::First]), MIN_RATIO);
    assert_eq!(
        layout.adjust_focused(Axis::Horizontal, false, STEP),
        Ok(false)
    );
    let unchanged = layout.clone();
    assert_eq!(
        layout.adjust_focused(Axis::Horizontal, true, ZERO_STEP),
        Ok(false)
    );
    assert_eq!(layout.adjust_focused(Axis::Vertical, true, STEP), Ok(false));
    assert_eq!(layout, unchanged);
    assert_eq!(
        Layout::default().adjust_focused(Axis::Horizontal, true, STEP),
        Ok(false)
    );
}

#[test]
fn explicit_split_paths_modify_only_the_requested_divider_and_reject_leaves_atomically() {
    let mut layout = grid();
    let chosen = SplitRatio::try_from(ODD_RATIO).unwrap();
    for path in [vec![], vec![SplitBranch::First], vec![SplitBranch::Second]] {
        assert_eq!(layout.resize_split(&path, chosen), Ok(true));
        assert_eq!(layout.resize_split(&path, chosen), Ok(false));
        assert_eq!(ratio_at(&layout, &path), ODD_RATIO);
    }
    let original = layout.clone();
    for invalid in [
        vec![SplitBranch::First, SplitBranch::First],
        vec![SplitBranch::Second, SplitBranch::Second, SplitBranch::First],
        vec![SplitBranch::First; MAX_LAYOUT_DEPTH + PATH_STEP],
    ] {
        assert_eq!(
            layout.resize_split(&invalid, SplitRatio::default()),
            Err(LayoutError::InvalidSplitPath)
        );
        assert_eq!(layout, original);
    }
    assert_eq!(
        Layout::default().resize_split(&[], chosen),
        Err(LayoutError::InvalidSplitPath)
    );
}

#[test]
fn balance_visits_both_subtrees_and_preserves_every_binding_and_focus() {
    for changed in [
        vec![],
        vec![vec![]],
        vec![vec![SplitBranch::First]],
        vec![vec![SplitBranch::Second]],
        vec![vec![], vec![SplitBranch::First], vec![SplitBranch::Second]],
    ] {
        let mut layout = grid();
        let original = layout.clone();
        for path in &changed {
            layout
                .resize_split(path, SplitRatio::try_from(MIN_RATIO).unwrap())
                .unwrap();
        }
        assert_eq!(layout.balance(), !changed.is_empty());
        assert_eq!(layout, original);
        assert!(!layout.balance());
    }
    assert!(!Layout::default().balance());
}
