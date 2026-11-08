use ratatui::text::Span;
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn boundaries(text: &str) -> Vec<usize> {
    let mut positions = vec![0];
    // Widget graphemes omit controls; their lengths cannot address raw values
    // such as a valid Unix directory containing a newline before a wide glyph.
    for (offset, grapheme) in text.grapheme_indices(true) {
        positions.push(offset + grapheme.len());
    }
    positions
}

pub(crate) fn prefix(text: &str, columns: usize) -> &str {
    let mut bytes = 0;
    let mut width = 0;
    for (offset, grapheme) in text.grapheme_indices(true) {
        let next = display_width(grapheme);
        if width + next > columns {
            break;
        }
        bytes = offset + grapheme.len();
        width += next;
    }
    &text[..bytes]
}

pub(crate) fn display_width(text: &str) -> usize {
    Span::raw(display(text)).width()
}

/// Prefilled values retain their exact bytes. Only their presentation escapes
/// controls, so editing never silently targets a different directory.
pub(crate) fn display(text: &str) -> Cow<'_, str> {
    if !text.chars().any(char::is_control) {
        return Cow::Borrowed(text);
    }
    let mut rendered = String::new();
    for character in text.chars() {
        if character.is_control() {
            rendered.extend(character.escape_default());
        } else {
            rendered.push(character);
        }
    }
    Cow::Owned(rendered)
}
