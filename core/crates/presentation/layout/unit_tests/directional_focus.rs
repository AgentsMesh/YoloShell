use crate::editing_fixture::*;
use yolo_presentation_layout::*;

#[test]
fn directional_focus_selects_aligned_neighbors_and_does_not_wrap_edges() {
    let transitions = [
        (TOP_LEFT, Direction::Right, TOP_RIGHT),
        (TOP_LEFT, Direction::Down, BOTTOM_LEFT),
        (TOP_RIGHT, Direction::Left, TOP_LEFT),
        (TOP_RIGHT, Direction::Down, BOTTOM_RIGHT),
        (BOTTOM_LEFT, Direction::Right, BOTTOM_RIGHT),
        (BOTTOM_LEFT, Direction::Up, TOP_LEFT),
        (BOTTOM_RIGHT, Direction::Left, BOTTOM_LEFT),
        (BOTTOM_RIGHT, Direction::Up, TOP_RIGHT),
    ];
    for (from, direction, to) in transitions {
        let mut layout = grid();
        layout.focus(id(from)).unwrap();
        let original = layout.document().root.clone();
        assert_eq!(layout.focus_direction(direction, bounds()), Ok(true));
        assert_eq!(layout.focused(), id(to));
        assert_eq!(layout.document().root, original);
    }
    for (from, direction) in [
        (TOP_LEFT, Direction::Left),
        (TOP_LEFT, Direction::Up),
        (TOP_RIGHT, Direction::Right),
        (BOTTOM_RIGHT, Direction::Down),
    ] {
        let mut layout = grid();
        layout.focus(id(from)).unwrap();
        let original = layout.clone();
        assert_eq!(layout.focus_direction(direction, bounds()), Ok(false));
        assert_eq!(layout, original);
    }
}

#[test]
fn equal_distance_neighbors_are_selected_deterministically_by_pane_identity() {
    let mut layout = Layout::try_from(PaneLayoutDocument {
        root: split(
            Axis::Horizontal,
            pane(TOP_LEFT),
            split(Axis::Vertical, pane(TOP_RIGHT), pane(BOTTOM_RIGHT)),
        ),
        focused: id(TOP_LEFT),
    })
    .unwrap();
    assert!(layout.focus_direction(Direction::Right, bounds()).unwrap());
    assert_eq!(layout.focused(), id(TOP_RIGHT));
    assert!(layout.focus_direction(Direction::Left, bounds()).unwrap());
    assert_eq!(layout.focused(), id(TOP_LEFT));
}

#[test]
fn collapsed_panes_are_neither_targets_nor_sources_and_invalid_bounds_are_atomic() {
    for dimensions in [(ZERO_STEP, HEIGHT), (WIDTH, ZERO_STEP), (CELL, CELL)] {
        let mut layout = grid();
        let original = layout.clone();
        let bounds = Rect {
            width: dimensions.0,
            height: dimensions.1,
            ..Rect::default()
        };
        for direction in [
            Direction::Left,
            Direction::Right,
            Direction::Up,
            Direction::Down,
        ] {
            assert_eq!(layout.focus_direction(direction, bounds), Ok(false));
            assert_eq!(layout, original);
        }
    }
    for invalid in [
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
        let mut layout = grid();
        let original = layout.clone();
        assert_eq!(
            layout.focus_direction(Direction::Right, invalid),
            Err(LayoutError::InvalidRect)
        );
        assert_eq!(layout, original);
    }
}

#[test]
fn zero_area_candidate_does_not_displace_the_visible_neighbor() {
    let mut layout = grid();
    layout.focus(id(BOTTOM_RIGHT)).unwrap();
    let bounds = Rect {
        width: WIDTH,
        height: CELL,
        ..Rect::default()
    };
    assert_eq!(layout.focus_direction(Direction::Left, bounds), Ok(true));
    assert_eq!(layout.focused(), id(BOTTOM_LEFT));
    assert_eq!(layout.focus_direction(Direction::Up, bounds), Ok(false));
}

#[test]
fn nearest_directional_edge_wins_over_a_farther_aligned_pane() {
    let mut layout = Layout::try_from(PaneLayoutDocument {
        root: split(
            Axis::Horizontal,
            pane(TOP_LEFT),
            split(Axis::Horizontal, pane(TOP_RIGHT), pane(BOTTOM_RIGHT)),
        ),
        focused: id(TOP_LEFT),
    })
    .unwrap();
    assert!(layout.focus_direction(Direction::Right, bounds()).unwrap());
    assert_eq!(layout.focused(), id(TOP_RIGHT));
    assert!(layout.focus_direction(Direction::Right, bounds()).unwrap());
    assert_eq!(layout.focused(), id(BOTTOM_RIGHT));
    assert!(layout.focus_direction(Direction::Left, bounds()).unwrap());
    assert_eq!(layout.focused(), id(TOP_RIGHT));
    let mut alone = Layout::default();
    let original = alone.clone();
    for direction in [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ] {
        assert_eq!(alone.focus_direction(direction, bounds()), Ok(false));
        assert_eq!(alone, original);
    }
}
