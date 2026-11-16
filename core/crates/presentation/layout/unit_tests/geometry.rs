use crate::fixture::*;
use yolo_presentation_layout::*;

#[test]
fn recursive_panes_partition_the_terminal_without_overlap_or_lost_cells() {
    let mut layout = Layout::new(Some(terminal()));
    let first = layout.focused();
    layout.split(Axis::Horizontal, Some(terminal())).unwrap();
    layout.split(Axis::Vertical, None).unwrap();
    layout.focus(first).unwrap();
    layout.split(Axis::Vertical, None).unwrap();
    for width in [0, 1, 2, 3, 79, 80, u16::MAX] {
        for height in [0, 1, 2, 23, 24, u16::MAX] {
            let bounds = Rect {
                x: 0,
                y: 0,
                width,
                height,
            };
            let rects = layout.geometry(bounds).unwrap();
            assert_eq!(rects.len(), layout.panes().len());
            let area: u64 = rects
                .iter()
                .map(|p| u64::from(p.rect.width) * u64::from(p.rect.height))
                .sum();
            assert_eq!(area, u64::from(width) * u64::from(height));
            for (index, a) in rects.iter().enumerate() {
                assert!(a.rect.x + a.rect.width <= width);
                assert!(a.rect.y + a.rect.height <= height);
                for b in &rects[index + 1..] {
                    assert!(
                        a.rect.width == 0
                            || a.rect.height == 0
                            || b.rect.width == 0
                            || b.rect.height == 0
                            || a.rect.x + a.rect.width <= b.rect.x
                            || b.rect.x + b.rect.width <= a.rect.x
                            || a.rect.y + a.rect.height <= b.rect.y
                            || b.rect.y + b.rect.height <= a.rect.y
                    );
                }
            }
        }
    }
}

#[test]
fn geometry_respects_offsets_and_refuses_coordinate_overflow() {
    let layout = Layout::default();
    let valid = Rect {
        x: 10,
        y: 20,
        width: 40,
        height: 30,
    };
    assert_eq!(layout.geometry(valid).unwrap()[0].rect, valid);
    for invalid in [
        Rect {
            x: u16::MAX,
            width: 1,
            ..Rect::default()
        },
        Rect {
            y: u16::MAX,
            height: 1,
            ..Rect::default()
        },
    ] {
        assert_eq!(layout.geometry(invalid), Err(LayoutError::InvalidRect));
    }
}
