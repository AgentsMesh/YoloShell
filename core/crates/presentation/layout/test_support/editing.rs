use uuid::Uuid;
use yolo_presentation_layout::*;

pub const TOP_LEFT: u128 = 1;
pub const TOP_RIGHT: u128 = 2;
pub const BOTTOM_LEFT: u128 = 3;
pub const BOTTOM_RIGHT: u128 = 4;
pub const OFFSET_X: u16 = 7;
pub const OFFSET_Y: u16 = 11;
pub const WIDTH: u16 = 80;
pub const HEIGHT: u16 = 40;
pub const HALF_WIDTH: u16 = 40;
pub const HALF_HEIGHT: u16 = 20;
pub const STEP: u16 = 50;
pub const ZERO_STEP: u16 = 0;
pub const ODD_RATIO: u16 = 333;
pub const CELL: u16 = 1;
pub const PATH_STEP: usize = 1;
pub const SPLIT_COUNT: usize = 3;
pub const EXTENTS: &[u16] = &[0, 1, 2, 3, 79, 80, u16::MAX];

pub fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

pub fn bounds() -> Rect {
    Rect {
        x: OFFSET_X,
        y: OFFSET_Y,
        width: WIDTH,
        height: HEIGHT,
    }
}

pub fn pane(value: u128) -> Tree {
    Tree::Pane(Pane {
        id: id(value),
        terminal: Some(crate::fixture::terminal()),
    })
}

pub fn split(axis: Axis, first: Tree, second: Tree) -> Tree {
    Tree::Split {
        axis,
        ratio: SplitRatio::default(),
        first: Box::new(first),
        second: Box::new(second),
    }
}

pub fn grid() -> Layout {
    Layout::try_from(PaneLayoutDocument {
        root: split(
            Axis::Horizontal,
            split(Axis::Vertical, pane(TOP_LEFT), pane(BOTTOM_LEFT)),
            split(Axis::Vertical, pane(TOP_RIGHT), pane(BOTTOM_RIGHT)),
        ),
        focused: id(TOP_LEFT),
    })
    .unwrap()
}

pub fn ratio_at(layout: &Layout, path: &[SplitBranch]) -> u16 {
    let mut tree = &layout.document().root;
    for branch in path {
        let Tree::Split { first, second, .. } = tree else {
            panic!()
        };
        tree = match branch {
            SplitBranch::First => first,
            SplitBranch::Second => second,
        };
    }
    let Tree::Split { ratio, .. } = tree else {
        panic!()
    };
    (*ratio).into()
}
