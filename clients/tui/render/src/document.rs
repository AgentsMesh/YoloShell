//! Read-only ANSI document pane. SGR styling is rendered; cursor movement and
//! other terminal controls are ignored because the content is static.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_document(frame: &mut Frame<'_>, title: &str, text: &str, scroll: u16, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_owned());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = parse(text);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((scroll.min(u16::MAX / 2), 0)),
        inner,
    );
}

/// Styled lines for a static ANSI document. Exposed for the rendering tests;
/// `render_document` is the production entry point.
pub fn styled_lines(text: &str) -> Vec<Line<'static>> {
    parse(text)
}

fn parse(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut style = Style::default();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\u{1b}' if chars.peek() == Some(&'[') => {
                chars.next();
                let mut parameters = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        if next == 'm' {
                            push(&mut spans, &mut buffer, style);
                            style = apply(style, &parameters);
                        }
                        break;
                    }
                    parameters.push(next);
                }
            }
            '\r' => {}
            '\n' => {
                push(&mut spans, &mut buffer, style);
                lines.push(Line::from(std::mem::take(&mut spans)));
            }
            other if !other.is_control() => buffer.push(other),
            _ => {}
        }
    }
    push(&mut spans, &mut buffer, style);
    if !spans.is_empty() || lines.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

fn push(spans: &mut Vec<Span<'static>>, buffer: &mut String, style: Style) {
    if !buffer.is_empty() {
        spans.push(Span::styled(std::mem::take(buffer), style));
    }
}

fn apply(mut style: Style, parameters: &str) -> Style {
    let codes = parameters
        .split(';')
        .map(|value| {
            if value.is_empty() {
                0
            } else {
                value.parse::<u16>().unwrap_or(0)
            }
        })
        .collect::<Vec<_>>();
    let mut index = 0;
    while index < codes.len() {
        let code = codes[index];
        match code {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            2 => style = style.add_modifier(Modifier::DIM),
            3 => style = style.add_modifier(Modifier::ITALIC),
            4 => style = style.add_modifier(Modifier::UNDERLINED),
            7 => style = style.add_modifier(Modifier::REVERSED),
            22 => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => style = style.remove_modifier(Modifier::ITALIC),
            24 => style = style.remove_modifier(Modifier::UNDERLINED),
            27 => style = style.remove_modifier(Modifier::REVERSED),
            30..=37 => style = style.fg(basic(code - 30)),
            39 => style = style.fg(Color::Reset),
            40..=47 => style = style.bg(basic(code - 40)),
            49 => style = style.bg(Color::Reset),
            90..=97 => style = style.fg(Color::Indexed((code - 90 + 8) as u8)),
            100..=107 => style = style.bg(Color::Indexed((code - 100 + 8) as u8)),
            38 | 48 => {
                if let Some((color, consumed)) = extended(&codes[index + 1..]) {
                    style = if code == 38 {
                        style.fg(color)
                    } else {
                        style.bg(color)
                    };
                    index += consumed;
                }
            }
            _ => {}
        }
        index += 1;
    }
    style
}

/// Parses `5;n` or `2;r;g;b` after an SGR 38/48 introducer. Returns the color
/// and how many parameter words it consumed.
fn extended(parameters: &[u16]) -> Option<(Color, usize)> {
    match parameters {
        [5, index, ..] => Some((Color::Indexed(u8::try_from(*index).ok()?), 2)),
        [2, red, green, blue, ..] => Some((
            Color::Rgb(
                u8::try_from(*red).ok()?,
                u8::try_from(*green).ok()?,
                u8::try_from(*blue).ok()?,
            ),
            4,
        )),
        _ => None,
    }
}

fn basic(index: u16) -> Color {
    match index {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        _ => Color::White,
    }
}
