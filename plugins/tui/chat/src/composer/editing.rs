use super::{Composer, DraftError};
use ratatui::{style::Style, text::Span};
use yolo_agent_input::MAX_PROMPT_BYTES;

impl Composer {
    pub fn insert(&mut self, text: &str) -> Result<bool, DraftError> {
        self.editable()?;
        if text
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
        {
            return Err(DraftError::UnsafeText);
        }
        if text.len() > MAX_PROMPT_BYTES.saturating_sub(self.text.len()) {
            return Err(DraftError::TooLarge);
        }
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.cursor = self
            .boundaries()
            .into_iter()
            .find(|position| *position >= self.cursor)
            .unwrap_or(self.text.len());
        Ok(!text.is_empty())
    }
    pub fn left(&mut self) {
        self.cursor = self
            .boundaries()
            .into_iter()
            .take_while(|position| *position < self.cursor)
            .last()
            .unwrap_or_default();
    }
    pub fn right(&mut self) {
        self.cursor = self
            .boundaries()
            .into_iter()
            .find(|position| *position > self.cursor)
            .unwrap_or(self.text.len());
    }
    pub fn home(&mut self) {
        self.cursor = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + '\n'.len_utf8());
    }
    pub fn document_start(&mut self) {
        self.cursor = 0;
    }
    pub fn document_end(&mut self) {
        self.cursor = self.text.len();
    }
    pub fn end(&mut self) {
        self.cursor += self.text[self.cursor..]
            .find('\n')
            .unwrap_or(self.text.len() - self.cursor);
    }
    pub fn backspace(&mut self) -> Result<bool, DraftError> {
        self.editable()?;
        let previous = self.cursor;
        self.left();
        self.text.replace_range(self.cursor..previous, "");
        Ok(self.cursor != previous)
    }
    pub fn delete(&mut self) -> Result<bool, DraftError> {
        self.editable()?;
        let previous = self.cursor;
        self.right();
        let next = self.cursor;
        self.text.replace_range(previous..next, "");
        self.cursor = previous;
        Ok(next != previous)
    }
    fn boundaries(&self) -> Vec<usize> {
        let mut positions = vec![0];
        let mut offset = 0;
        for part in self.text.split_inclusive(['\n', '\t']) {
            let visible = part.trim_end_matches(['\n', '\t']);
            let span = Span::raw(visible);
            for grapheme in span.styled_graphemes(Style::default()) {
                offset += grapheme.symbol.len();
                positions.push(offset);
            }
            if part.len() > visible.len() {
                offset += part.len() - visible.len();
                positions.push(offset);
            }
        }
        positions
    }
}
