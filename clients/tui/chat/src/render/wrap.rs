use crate::constants::*;
use ratatui::{style::Style, text::Span};

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

pub(super) fn composer_rows(text: &str, width: u16) -> Vec<String> {
    let mut rows = Vec::new();
    for_each_row(text, width, |row| {
        rows.push(row);
        rows.len() < MAX_COMPOSER_ROWS
    });
    rows
}

/// Emits sanitized rows until the consumer has enough.
pub(super) fn for_each_row(text: &str, width: u16, mut emit: impl FnMut(String) -> bool) {
    if width == 0 {
        return;
    }
    for source in text.split('\n') {
        let sanitized = safe(&source.replace('\t', TAB_REPLACEMENT));
        let span = Span::raw(sanitized.as_str());
        let mut line = String::new();
        let mut used = 0;
        for grapheme in span.styled_graphemes(Style::default()) {
            let size = Span::raw(grapheme.symbol).width();
            if used + size > usize::from(width) && !line.is_empty() {
                if !emit(std::mem::take(&mut line)) {
                    return;
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
        if !emit(line) {
            return;
        }
    }
}
