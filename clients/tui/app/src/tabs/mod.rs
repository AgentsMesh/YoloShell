mod actions;
pub(crate) mod close;
pub(crate) mod creation;
pub(crate) mod manager;
mod navigation;
mod pointer;
pub(crate) mod strip;
pub(crate) mod terminal_picker;
pub(crate) use actions::{prefix, sidebar};
pub(crate) use navigation::{activate, open};
pub(crate) use pointer::click;

pub(crate) const TAB_BAR_ROWS: u16 = 1;
pub(crate) const SIDEBAR_COLUMNS: u16 = 36;
pub(crate) const SIDEBAR_DIVIDER_COLUMNS: u16 = 1;
pub(crate) const MIN_CONTENT_COLUMNS: u16 = 40;
pub(crate) const MANAGER_KEY: char = 'w';
pub(crate) const PICKER_KEY: char = 'g';
pub(crate) const TOGGLE_SIDEBAR_KEY: char = 'b';
pub(crate) const NEXT_KEY: char = 'n';
pub(crate) const PREVIOUS_KEY: char = 'p';
pub(crate) const SPLIT_RIGHT_KEY: char = 'v';
pub(crate) const SPLIT_DOWN_KEY: char = '-';
pub(crate) const CLOSE_TAB_KEY: char = 'X';
pub(crate) const RESUME_KEY: char = 'R';
pub(crate) const DETAILS_KEY: char = 'I';
pub(crate) const TERMINAL_PICKER_TITLE: &str = "Terminals";
pub(crate) const LITERAL_PREFIX: u8 = 0x02;
/// Chat mode is valid only while a chat entry exists for the focused pane.
/// A picker restore, a pane/tab close or a background binding retirement can
/// remove that entry; without this invariant every key is swallowed with no
/// visible overlay.
pub(crate) fn settle_chat_mode(state: &mut crate::State) {
    if state.mode == crate::Mode::Chat && crate::chat::current(state).is_none() {
        state.mode = crate::Mode::Terminal;
    }
}

pub(crate) const LAYOUT_INVARIANT: &str =
    "validated tab and focused pane remain present during a local transition";
