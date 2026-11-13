mod editing;
mod graphemes;
mod render;
mod viewport;

pub use render::render;
pub use viewport::Viewport;

/// Editing position in a caller-owned, single-line value. A reset follows the
/// end of a replacement value; every operation also normalizes byte boundaries.
#[derive(Clone, Debug, Default)]
pub struct Cursor {
    offset: Option<usize>,
    select_all: bool,
}

impl Cursor {
    pub fn reset(&mut self) {
        self.offset = None;
        self.select_all = false;
    }

    /// A replacement form can prefill the old value while making the first
    /// edit replace it wholesale, like a selected text field.
    pub fn select_all(&mut self) {
        self.offset = Some(0);
        self.select_all = true;
    }

    pub fn position(&self, text: &str) -> usize {
        let requested = self.offset.unwrap_or(text.len()).min(text.len());
        graphemes::boundaries(text)
            .into_iter()
            .take_while(|position| *position <= requested)
            .last()
            .unwrap_or_default()
    }

    fn left(&self, text: &str) -> usize {
        let current = self.position(text);
        graphemes::boundaries(text)
            .into_iter()
            .take_while(|position| *position < current)
            .last()
            .unwrap_or_default()
    }

    fn right(&self, text: &str) -> usize {
        let current = self.position(text);
        graphemes::boundaries(text)
            .into_iter()
            .find(|position| *position > current)
            .unwrap_or(text.len())
    }
}
