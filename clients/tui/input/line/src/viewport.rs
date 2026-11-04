use crate::{
    Cursor,
    graphemes::{boundaries, display, display_width, prefix},
};
use std::borrow::Cow;

const CURSOR_COLUMNS: usize = 1;

#[derive(Debug, Eq, PartialEq)]
pub struct Viewport<'a> {
    pub text: Cow<'a, str>,
    pub cursor_column: Option<u16>,
}

impl Cursor {
    /// Keep the cursor visible without cutting a wide or combining grapheme.
    pub fn viewport<'a>(&self, text: &'a str, columns: u16) -> Viewport<'a> {
        if columns == 0 {
            return Viewport {
                text: Cow::Borrowed(""),
                cursor_column: None,
            };
        }
        let cursor = self.position(text);
        let available = usize::from(columns).saturating_sub(CURSOR_COLUMNS);
        let mut start = cursor;
        for position in boundaries(text)
            .into_iter()
            .rev()
            .filter(|offset| *offset < cursor)
        {
            if display_width(&text[position..cursor]) > available {
                break;
            }
            start = position;
        }
        Viewport {
            text: display(prefix(&text[start..], usize::from(columns))),
            cursor_column: Some(display_width(&text[start..cursor]) as u16),
        }
    }
}
