use crate::constants::*;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub(super) fn safe(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                CONTROL_REPLACEMENT
            } else {
                character
            }
        })
        .collect()
}

/// Every output row is safe to print directly; transcript ANSI never reaches the terminal.
pub(super) fn wrap(text: &str, width: u16, limit: usize) -> (Vec<String>, bool) {
    if width == 0 || limit == 0 {
        return (Vec::new(), !text.is_empty());
    }
    let mut rows = Vec::new();
    for source in text.split('\n') {
        let sanitized = safe(&source.replace('\t', TAB_REPLACEMENT));
        let span = Span::raw(sanitized.as_str());
        let mut line = String::new();
        let mut used = 0;
        for grapheme in span.styled_graphemes(Style::default()) {
            let size = Line::raw(grapheme.symbol).width();
            if used + size > usize::from(width) && !line.is_empty() {
                rows.push(std::mem::take(&mut line));
                if rows.len() == limit {
                    return (rows, true);
                }
                used = 0;
            }
            if size <= usize::from(width) {
                line.push_str(grapheme.symbol);
                used += size;
            } else {
                line.push(CONTROL_REPLACEMENT);
                used += usize::from(CELL);
            }
        }
        if rows.len() == limit {
            return (rows, true);
        }
        rows.push(line);
    }
    (rows, false)
}
