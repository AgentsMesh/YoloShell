use crate::constants::*;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, widgets::Widget};
use std::collections::BTreeMap;
use yolo_tui_render::{
    PaneSelection, PaneViewport, TerminalWidget, highlight_selection, render_terminal,
};

#[test]
fn a_viewport_from_a_larger_window_highlights_only_the_current_frame_intersection() {
    let pane = uuid::Uuid::new_v4();
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    let view = PaneViewport {
        pane,
        terminal: None,
        outer: Rect::new(0, 0, COLUMNS, ROWS),
        inner: Rect::new(
            COLUMNS - INNER_INSET,
            ROWS - INNER_INSET,
            SHORT_EXTENT,
            SHORT_EXTENT,
        ),
    };
    terminal
        .draw(|frame| {
            highlight_selection(
                frame,
                view,
                PaneSelection {
                    pane,
                    start: (0, 0),
                    end: (SHORT_EXTENT, SHORT_EXTENT),
                },
            )
        })
        .unwrap();
    let selected = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .filter(|cell| cell.bg == SELECTED_BACKGROUND)
        .count();
    assert_eq!(selected, SINGLE_VIEWPORT);
    assert_eq!(
        terminal.backend().buffer()[(COLUMNS - INNER_INSET, ROWS - INNER_INSET)].bg,
        SELECTED_BACKGROUND
    );
}

#[test]
fn empty_or_outside_viewports_and_far_selection_endpoints_do_not_expand_frame_access() {
    let pane = uuid::Uuid::new_v4();
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    for inner in [
        Rect::new(INNER_INSET, INNER_INSET, SHORT_EXTENT, 0),
        Rect::new(INNER_INSET, INNER_INSET, 0, SHORT_EXTENT),
        Rect::new(COLUMNS, ROWS, SHORT_EXTENT, SHORT_EXTENT),
        Rect::new(
            u16::MAX - INNER_INSET,
            u16::MAX - INNER_INSET,
            SHORT_EXTENT,
            SHORT_EXTENT,
        ),
    ] {
        terminal
            .draw(|frame| {
                let view = PaneViewport {
                    pane,
                    terminal: None,
                    outer: frame.area(),
                    inner,
                };
                highlight_selection(
                    frame,
                    view,
                    PaneSelection {
                        pane,
                        start: (0, 0),
                        end: (u16::MAX, u16::MAX),
                    },
                );
            })
            .unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .all(|cell| cell.bg != SELECTED_BACKGROUND)
        );
    }
}

#[test]
fn malformed_tab_bounds_fail_before_any_frame_access() {
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    for bounds in [
        Rect {
            x: u16::MAX,
            y: 0,
            width: INNER_INSET,
            height: ROWS,
        },
        Rect {
            x: 0,
            y: u16::MAX,
            width: COLUMNS,
            height: INNER_INSET,
        },
    ] {
        terminal
            .draw(|frame| {
                assert_eq!(
                    render_terminal(
                        frame,
                        &yolo_presentation_layout::Layout::default(),
                        &BTreeMap::new(),
                        bounds,
                        None
                    ),
                    Err(yolo_presentation_layout::LayoutError::InvalidRect)
                );
            })
            .unwrap();
    }
}

#[test]
fn resized_scrollback_rows_with_their_original_narrow_width_render_without_invented_cells() {
    let mut parser = vt100::Parser::new(ROWS, NARROW_COLUMNS, SCROLLBACK_ROWS);
    parser.process(&SCROLLED_LINE.repeat(usize::from(ROWS) + HISTORY_STEP));
    parser.screen_mut().set_size(ROWS, COLUMNS);
    parser.screen_mut().set_scrollback(HISTORY_STEP);
    assert!(parser.screen().cell(0, NARROW_COLUMNS).is_none());
    let area = Rect::new(0, 0, COLUMNS, ROWS);
    let mut buffer = Buffer::empty(area);
    TerminalWidget(parser.screen()).render(area, &mut buffer);
    let text = (0..NARROW_COLUMNS)
        .map(|column| buffer[(column, 0)].symbol())
        .collect::<String>();
    assert!(text.starts_with(SCROLLED_TEXT));
    for column in NARROW_COLUMNS..COLUMNS {
        assert_eq!(buffer[(column, 0)].symbol(), BLANK_CELL);
    }
}
