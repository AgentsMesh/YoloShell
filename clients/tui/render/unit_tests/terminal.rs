use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};
use std::collections::BTreeMap;
use yolo_presentation_layout::{Axis, Layout};
use yolo_tui_render::{TerminalPane, render_terminal, terminal_inner};

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const CELL: u16 = 1;
const CONTENT: &[u8] = b"terminal owns the first cell\r\n";

#[test]
fn single_terminal_uses_every_cell_without_frame_or_title_and_places_source_cursor() {
    let address = yolo_hub_identity::TerminalAddress {
        hub_id: yolo_hub_identity::HubId::new(),
        terminal_id: yolo_hub_identity::TerminalId::new(),
    };
    let layout = Layout::new(Some(address));
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    parser.process(CONTENT);
    let terminals = BTreeMap::from([(
        address,
        TerminalPane {
            screen: Some(parser.screen()),
            projection: crate::constants::WRITING,
        },
    )]);
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    terminal
        .draw(|frame| {
            let bounds = frame.area();
            let views = render_terminal(frame, &layout, &terminals, bounds, None).unwrap();
            assert_eq!(views.len(), usize::from(CELL));
            assert_eq!(views[0].inner, bounds);
            assert_eq!(views[0].outer, bounds);
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "t");
    assert_eq!(
        terminal.backend().buffer()[(COLUMNS - CELL, ROWS - CELL)].symbol(),
        " "
    );
}

#[test]
fn ended_terminal_shows_the_action_card_over_the_retained_screen() {
    let address = yolo_hub_identity::TerminalAddress {
        hub_id: yolo_hub_identity::HubId::new(),
        terminal_id: yolo_hub_identity::TerminalId::new(),
    };
    let layout = Layout::new(Some(address));
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    parser.process(b"RETAINED SCREEN");
    let projection = yolo_tui_lifecycle::TerminalProjection::new(
        yolo_tui_lifecycle::TerminalPhase::Ended,
        yolo_tui_lifecycle::ConnectionState::Connected,
        false,
        true,
    );
    let terminals = BTreeMap::from([(
        address,
        TerminalPane {
            screen: Some(parser.screen()),
            projection,
        },
    )]);
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    terminal
        .draw(|frame| {
            render_terminal(frame, &layout, &terminals, frame.area(), None).unwrap();
        })
        .unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(text.contains("RETAINED SCREEN"));
    assert!(text.contains("Ended"));
    assert!(text.contains("Ctrl-b c: new terminal"));
}

#[test]
fn nested_focus_highlights_only_adjacent_divider_segments_and_has_no_outer_border() {
    let mut layout = Layout::default();
    let top_left = layout.focused();
    let top_right = layout.split(Axis::Horizontal, None).unwrap();
    layout.focus(top_left).unwrap();
    let bottom_left = layout.split(Axis::Vertical, None).unwrap();
    layout.focus(top_right).unwrap();
    layout.split(Axis::Vertical, None).unwrap();
    layout.focus(bottom_left).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    let mut views = Vec::new();
    terminal
        .draw(|frame| {
            views = render_terminal(frame, &layout, &BTreeMap::new(), frame.area(), None).unwrap()
        })
        .unwrap();
    let focused = views.iter().find(|view| view.pane == bottom_left).unwrap();
    let divider = focused.outer.right();
    assert_eq!(
        terminal.backend().buffer()[(divider, focused.outer.y - CELL)].fg,
        Color::DarkGray
    );
    assert_eq!(
        terminal.backend().buffer()[(divider, focused.outer.bottom() - CELL)].fg,
        Color::Cyan
    );
    assert_eq!(
        terminal.backend().buffer()[(COLUMNS - CELL, 0)].symbol(),
        " "
    );
    assert_eq!(focused.inner.x, focused.outer.x);
    assert_eq!(focused.inner.y, focused.outer.y + CELL);
}

#[test]
fn tiny_and_offset_terminals_keep_dividers_inside_bounds_without_inventing_content_cells() {
    let mut layout = Layout::default();
    layout.split(Axis::Horizontal, None).unwrap();
    layout.split(Axis::Vertical, None).unwrap();
    for (width, height) in [(0, 0), (1, 1), (2, 2), (3, 1), (1, 3)] {
        let bounds = Rect::new(CELL, CELL, width, height);
        let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
        terminal
            .draw(|frame| {
                for view in render_terminal(frame, &layout, &BTreeMap::new(), bounds, None).unwrap()
                {
                    assert_eq!(view.inner, terminal_inner(view.outer, bounds));
                    assert!(view.inner.right() <= view.outer.right());
                    assert!(view.inner.bottom() <= view.outer.bottom());
                }
            })
            .unwrap();
    }
}
