use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Materialize rows before layout so pagination and error space use the same
/// grapheme boundaries as rendering, without hidden widget wrapping state.
pub fn wrap(text: &str, width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let mut result = Vec::new();
    for source in text.lines() {
        let mut row = String::new();
        let mut used = 0;
        for grapheme in Span::raw(source).styled_graphemes(Style::default()) {
            let columns = Line::raw(grapheme.symbol).width();
            if used + columns > usize::from(width) && !row.is_empty() {
                result.push(Line::raw(std::mem::take(&mut row)));
                used = 0;
            }
            row.push_str(grapheme.symbol);
            used += columns;
        }
        result.push(Line::raw(row));
    }
    result
}
