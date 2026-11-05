use crate::{Mode, State};
use ratatui::layout::Rect;
use uuid::Uuid;
use yolo_hub_identity::RuntimeTarget;
use yolo_presentation_layout::Layout;
use yolo_tui_selection::{ClipboardCommand, Selection};

pub(crate) const COPY_KEY: char = 'y';
pub(crate) const SELECTING_LABEL: &str = "Selection paused · release, then Ctrl+B y to copy";
pub(crate) const SELECTED_LABEL: &str = "Selection paused · Ctrl+B y copies · Esc resumes";
pub(crate) const COPY_QUEUED_LABEL: &str = "Selection queued for terminal clipboard";
pub(crate) const CLEARED_LABEL: &str = "Selection cleared";
pub(crate) const NO_SELECTION_LABEL: &str = "Shift+drag over terminal text before copying";

#[derive(Default)]
pub(crate) struct SelectionState {
    pub active: Option<BoundSelection>,
    pub pending: Option<ClipboardCommand>,
}

pub(crate) struct BoundSelection {
    pub source: Source,
    pub text: Selection,
}

pub(crate) struct Source {
    pub pane: Uuid,
    pub target: RuntimeTarget,
    pub inner: Rect,
    pub size: (u16, u16),
    pub scrollback: usize,
    pub layout: Layout,
    pub mode: Mode,
    pub zoomed: bool,
}

impl Source {
    pub fn current(&self, state: &State) -> bool {
        state.mode == self.mode
            && state.hub_manager.is_none()
            && state.tab_manager.is_none()
            && *state.tab_layout() == self.layout
            && state.tab_layout().focused() == self.pane
            && state.pane_view.zoomed == self.zoomed
            && state.presentation.zoomed == self.zoomed
            && crate::observation_events::current(state, self.target)
            && state.presentation.panes.iter().any(|view| {
                view.pane == self.pane
                    && view.terminal == Some(self.target.address)
                    && view.inner == self.inner
            })
            && state
                .surfaces
                .get(&self.target.address)
                .is_some_and(|surface| {
                    surface.cursor().target == self.target
                        && surface.screen().size() == self.size
                        && surface.screen().scrollback() == self.scrollback
                })
    }
}
