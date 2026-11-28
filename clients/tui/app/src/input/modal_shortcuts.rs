use crate::{
    Mode, State,
    help_constants::{DETACH_KEY, QUIT_KEY},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

/// The application owns the detach prefix in every modal; form editors retain
/// ordinary characters, including q, and cannot accidentally dispatch PTY input.
pub(crate) fn handle(state: &mut State, event: &Event) -> Option<bool> {
    let modal = state.help
        || state.tab_manager.is_some()
        || matches!(
            state.mode,
            Mode::Directory
                | Mode::Search
                | Mode::ConfirmTerminate
                | Mode::ConfirmArchive
                | Mode::ConfirmRecreate
                | Mode::HubConfiguration
        );
    if !modal {
        return None;
    }
    let Event::Key(key) = event else { return None };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if state.prefix {
        state.prefix = false;
        return Some(
            key.modifiers.is_empty() && matches!(key.code, KeyCode::Char(DETACH_KEY | QUIT_KEY)),
        );
    }
    if key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL {
        state.prefix = true;
        return Some(false);
    }
    None
}
