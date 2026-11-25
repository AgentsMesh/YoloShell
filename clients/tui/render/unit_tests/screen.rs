use crate::constants::*;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
    widgets::Widget,
};
use yolo_tui_render::TerminalWidget;

#[test]
fn vt_cells_preserve_colors_all_supported_attributes_and_combining_marks() {
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    parser.process(ATTRIBUTES);
    let area = Rect::new(0, 0, COLUMNS, ROWS);
    let mut buffer = Buffer::empty(area);
    TerminalWidget(parser.screen()).render(area, &mut buffer);
    let cell = &buffer[(0, 0)];
    assert_eq!(cell.symbol(), "X");
    assert_eq!(cell.fg, Color::Indexed(INDEXED_RED));
    assert_eq!(cell.bg, Color::Rgb(1, 2, 3));
    assert!(
        cell.modifier
            .contains(Modifier::DIM | Modifier::ITALIC | Modifier::UNDERLINED | Modifier::REVERSED)
    );
    assert!(!cell.modifier.contains(Modifier::BOLD));
    parser.process(b"\x1b[0m");
    parser.process(COMBINED_TEXT.as_bytes());
    TerminalWidget(parser.screen()).render(area, &mut buffer);
    assert_eq!(buffer[(1, 0)].symbol(), COMBINED_TEXT);
    assert_eq!(buffer[(1, 0)].fg, Color::Reset);
}

#[test]
fn bold_intensity_is_rendered_independently_of_dim() {
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    parser.process(BOLD_TEXT);
    let area = Rect::new(0, 0, COLUMNS, ROWS);
    let mut buffer = Buffer::empty(area);
    TerminalWidget(parser.screen()).render(area, &mut buffer);
    assert!(buffer[(0, 0)].modifier.contains(Modifier::BOLD));
    assert!(!buffer[(0, 0)].modifier.contains(Modifier::DIM));
}

#[test]
fn wide_cells_are_not_redrawn_as_independent_spaces_or_clipped_into_neighbor_panes() {
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    parser.process(WIDE_TEXT.as_bytes());
    let area = Rect::new(0, 0, COLUMNS, ROWS);
    let mut buffer = Buffer::empty(area);
    TerminalWidget(parser.screen()).render(area, &mut buffer);
    assert_eq!(buffer[(0, 0)].symbol(), WIDE_TEXT);
    assert!(buffer[(1, 0)].skip);
    let clipped = Rect::new(0, 0, 1, 1);
    TerminalWidget(parser.screen()).render(clipped, &mut buffer);
    assert_eq!(buffer[(0, 0)].symbol(), " ");
    TerminalWidget(parser.screen()).render(Rect::new(0, 0, 0, 0), &mut buffer);
}
