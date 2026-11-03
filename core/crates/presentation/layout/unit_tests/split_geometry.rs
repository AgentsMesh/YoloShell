use crate::editing_fixture::*;
use yolo_presentation_layout::*;

#[test]
fn split_geometry_uses_absolute_coordinates_and_preorder_branch_paths() {
    let layout = grid();
    let bounds = bounds();
    let geometry = layout.split_geometry(bounds).unwrap();
    assert_eq!(
        geometry,
        vec![
            SplitGeometry {
                path: vec![],
                axis: Axis::Horizontal,
                rect: bounds,
                boundary: OFFSET_X + HALF_WIDTH,
                ratio: SplitRatio::default(),
            },
            SplitGeometry {
                path: vec![SplitBranch::First],
                axis: Axis::Vertical,
                rect: Rect {
                    width: HALF_WIDTH,
                    ..bounds
                },
                boundary: OFFSET_Y + HALF_HEIGHT,
                ratio: SplitRatio::default(),
            },
            SplitGeometry {
                path: vec![SplitBranch::Second],
                axis: Axis::Vertical,
                rect: Rect {
                    x: OFFSET_X + HALF_WIDTH,
                    width: HALF_WIDTH,
                    ..bounds
                },
                boundary: OFFSET_Y + HALF_HEIGHT,
                ratio: SplitRatio::default(),
            },
        ]
    );
    assert!(Layout::default().split_geometry(bounds).unwrap().is_empty());
}

#[test]
fn divider_boundaries_match_the_same_integer_partition_as_pane_rectangles() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut layout = Layout::try_from(PaneLayoutDocument {
            root: split(axis, pane(TOP_LEFT), pane(TOP_RIGHT)),
            focused: id(TOP_LEFT),
        })
        .unwrap();
        for ratio in [MIN_RATIO, ODD_RATIO, MAX_RATIO] {
            layout
                .resize_split(&[], SplitRatio::try_from(ratio).unwrap())
                .unwrap();
            for span in EXTENTS {
                let bounds = Rect {
                    width: *span,
                    height: *span,
                    ..Rect::default()
                };
                let panes = layout.geometry(bounds).unwrap();
                let splits = layout.split_geometry(bounds).unwrap();
                let second = panes
                    .iter()
                    .find(|pane| pane.pane == id(TOP_RIGHT))
                    .unwrap();
                let actual = match axis {
                    Axis::Horizontal => second.rect.x,
                    Axis::Vertical => second.rect.y,
                };
                assert_eq!(splits.first().unwrap().boundary, actual);
                assert_eq!(splits.first().unwrap().rect, bounds);
                assert_eq!(u16::from(splits.first().unwrap().ratio), ratio);
            }
        }
    }
}

#[test]
fn invalid_rectangles_are_rejected_before_collecting_any_dividers() {
    let layout = grid();
    for bounds in [
        Rect {
            x: u16::MAX,
            width: CELL,
            ..Rect::default()
        },
        Rect {
            y: u16::MAX,
            height: CELL,
            ..Rect::default()
        },
    ] {
        assert_eq!(layout.split_geometry(bounds), Err(LayoutError::InvalidRect));
    }
}
