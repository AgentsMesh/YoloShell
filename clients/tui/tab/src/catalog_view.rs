use crate::{DocumentPane, LocalTabView, TabCatalog, TabViewMode, constants::*};

impl TabCatalog {
    pub fn update_active(&mut self, view: LocalTabView) {
        self.0
            .entries
            .iter_mut()
            .find(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR)
            .view = view;
    }

    pub fn active_layout(&self) -> &yolo_presentation_layout::Layout {
        &self.active().view.layout
    }

    pub fn active_layout_mut(&mut self) -> &mut yolo_presentation_layout::Layout {
        &mut self
            .0
            .entries
            .iter_mut()
            .find(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR)
            .view
            .layout
    }

    pub fn replace_active_layout(&mut self, layout: yolo_presentation_layout::Layout) {
        self.active_layout_mut().clone_from(&layout);
    }

    pub fn active_view(&self) -> TabViewMode {
        self.active().view.view
    }

    /// Document panes are client-local and keyed by their owning tab. They are
    /// not part of the shared layout, so publishing or adopting a layout never
    /// changes the terminal topology.
    pub fn active_document(&self) -> Option<&DocumentPane> {
        self.0.documents.get(&self.0.active)
    }

    pub fn open_document(&mut self, title: String, text: String) -> bool {
        self.0
            .documents
            .insert(self.0.active, DocumentPane { title, text });
        true
    }

    pub fn close_document(&mut self) -> bool {
        self.0.documents.remove(&self.0.active).is_some()
    }

    pub fn replace_active_view(&mut self, view: TabViewMode) {
        self.0
            .entries
            .iter_mut()
            .find(|entry| entry.id == self.0.active)
            .expect(ACTIVE_ERROR)
            .view
            .view = view;
    }
}
