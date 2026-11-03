use crate::{Cursor, graphemes::boundaries};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

impl Cursor {
    /// Returns whether the text changed; navigation may still move the cursor.
    /// Enter/Escape are left to the form owner and can never be pasted as actions.
    pub fn handle(&mut self, text: &mut String, event: &Event, max_bytes: usize) -> bool {
        if let Event::Paste(value) = event {
            return self.insert(text, value, max_bytes);
        }
        let Event::Key(key) = event else { return false };
        if key.kind != KeyEventKind::Press {
            return false;
        }
        let position = self.position(text);
        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Char('a') => {
                    self.select_all = false;
                    self.offset = Some(0);
                }
                KeyCode::Char('e') => {
                    self.select_all = false;
                    self.offset = Some(text.len());
                }
                KeyCode::Char('u') => {
                    if self.select_all {
                        self.select_all = false;
                        let changed = !text.is_empty();
                        text.clear();
                        self.offset = Some(0);
                        return changed;
                    }
                    text.replace_range(..position, "");
                    self.offset = Some(0);
                    return position != 0;
                }
                KeyCode::Char('k') => {
                    let changed = position != text.len();
                    text.truncate(position);
                    return changed;
                }
                _ => {}
            }
            return false;
        }
        if !key.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
            return false;
        }
        match key.code {
            KeyCode::Home => {
                self.select_all = false;
                self.offset = Some(0);
            }
            KeyCode::End => {
                self.select_all = false;
                self.offset = Some(text.len());
            }
            KeyCode::Left => {
                self.select_all = false;
                self.offset = Some(self.left(text));
            }
            KeyCode::Right => {
                self.select_all = false;
                self.offset = Some(self.right(text));
            }
            KeyCode::Backspace | KeyCode::Delete if self.select_all => {
                let changed = !text.is_empty();
                text.clear();
                self.offset = Some(0);
                self.select_all = false;
                return changed;
            }
            KeyCode::Backspace => {
                let previous = self.left(text);
                text.replace_range(previous..position, "");
                self.offset = Some(previous);
                return previous != position;
            }
            KeyCode::Delete => {
                let next = self.right(text);
                text.replace_range(position..next, "");
                self.offset = Some(position);
                return next != position;
            }
            KeyCode::Char(character) if !character.is_control() => {
                return self.insert(text, &character.to_string(), max_bytes);
            }
            _ => {}
        }
        false
    }

    fn insert(&mut self, text: &mut String, value: &str, max_bytes: usize) -> bool {
        let replaced = std::mem::take(&mut self.select_all);
        if replaced {
            text.clear();
            self.offset = Some(0);
        }
        let available = max_bytes.saturating_sub(text.len());
        if available == 0 {
            return replaced;
        }
        // One additional non-control scalar determines whether the last in-budget
        // grapheme continues. Do not copy or segment a potentially huge tail.
        let capacity = available
            .min(value.len())
            .saturating_add(char::MAX.len_utf8());
        let mut clean = String::with_capacity(capacity);
        for character in value.chars().filter(|value| !value.is_control()) {
            clean.push(character);
            if clean.len() > available {
                break;
            }
        }
        let mut bytes = 0;
        for (offset, grapheme) in clean.grapheme_indices(true) {
            if offset + grapheme.len() > available {
                break;
            }
            bytes = offset + grapheme.len();
        }
        let position = self.position(text);
        text.insert_str(position, &clean[..bytes]);
        self.offset = Some(
            boundaries(text)
                .into_iter()
                .find(|boundary| *boundary >= position + bytes)
                .unwrap_or(text.len()),
        );
        replaced || bytes != 0
    }
}
