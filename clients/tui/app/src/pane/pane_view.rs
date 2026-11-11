use crate::State;
use std::borrow::Cow;
use yolo_presentation_layout::Layout;

pub(crate) const ZOOM_KEY: char = 'z';
pub(crate) const SWAP_PREVIOUS_KEY: char = '{';
pub(crate) const SWAP_NEXT_KEY: char = '}';
const SINGLE_PANE: usize = 1;

/// View state only: never serialized as a replacement for the owning layout.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PaneView {
    pub zoomed: bool,
}

impl State {
    pub(crate) fn display_layout(&self) -> Cow<'_, Layout> {
        if self.pane_view.zoomed {
            Cow::Owned(self.tab_layout().focused_view())
        } else {
            Cow::Borrowed(self.tab_layout())
        }
    }

    pub(crate) fn toggle_pane_zoom(&mut self) {
        self.pane_view.zoomed =
            !self.pane_view.zoomed && self.tab_layout().panes().len() > SINGLE_PANE;
        self.pointer = Default::default();
    }
}
