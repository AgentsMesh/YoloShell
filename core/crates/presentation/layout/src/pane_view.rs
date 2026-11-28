use crate::{Layout, PaneLayoutDocument, Tree, constants::FOCUSED_PANE_INVARIANT};

impl Layout {
    /// A transient view that retains the focused pane/terminal identity. The full
    /// owning layout remains the source of persistence and runtime bindings.
    pub fn focused_view(&self) -> Self {
        let pane = self
            .panes()
            .into_iter()
            .find(|pane| pane.id == self.focused())
            .expect(FOCUSED_PANE_INVARIANT)
            .clone();
        Self(PaneLayoutDocument {
            root: Tree::Pane(pane),
            focused: self.focused(),
        })
    }
}
