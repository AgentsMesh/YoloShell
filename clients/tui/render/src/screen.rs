use crate::constants::{CELL_WIDTH, EMPTY_CELL};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct TerminalWidget<'a>(pub &'a vt100::Screen);

impl Widget for TerminalWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let (rows, columns) = self.0.size();
        for row in 0..area.height.min(rows) {
            for column in 0..area.width.min(columns) {
                let Some(source) = self.0.cell(row, column) else {
                    continue;
                };
                let target = &mut buffer[(area.x + column, area.y + row)];
                target.reset();
                target.set_style(style(source));
                if source.is_wide_continuation() {
                    target.set_skip(true);
                } else if source.is_wide() && column + CELL_WIDTH >= area.width {
                    target.set_symbol(EMPTY_CELL);
                } else {
                    target.set_symbol(if source.has_contents() {
                        source.contents()
                    } else {
                        EMPTY_CELL
                    });
                }
            }
        }
    }
}

fn style(cell: &vt100::Cell) -> Style {
    let mut modifiers = Modifier::empty();
    for (enabled, modifier) in [
        (cell.bold(), Modifier::BOLD),
        (cell.dim(), Modifier::DIM),
        (cell.italic(), Modifier::ITALIC),
        (cell.underline(), Modifier::UNDERLINED),
        (cell.inverse(), Modifier::REVERSED),
    ] {
        if enabled {
            modifiers |= modifier;
        }
    }
    Style::default()
        .fg(color(cell.fgcolor()))
        .bg(color(cell.bgcolor()))
        .add_modifier(modifiers)
}

fn color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(index) => Color::Indexed(index),
        vt100::Color::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
    }
}
