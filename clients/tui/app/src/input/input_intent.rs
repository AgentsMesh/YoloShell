use crate::{Mode, State};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use yolo_hub_identity::TerminalAddress;

/// Terminal metadata is not navigation. Typing while an exact initial grant is
/// pending keeps that acquisition alive; payload bytes are buffered against
/// that exact intent until its writer grant is confirmed.
pub(crate) fn changes_context(state: &State, event: &Event) -> bool {
    match event {
        Event::Resize(_, _) | Event::FocusGained | Event::FocusLost => false,
        Event::Key(key) if key.kind == KeyEventKind::Release => false,
        Event::Key(key) if waiting_in_terminal(state) && !state.prefix => {
            key.code == KeyCode::F(1)
                || (key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL)
                || (key.code == KeyCode::Esc && state.selection.active.is_some())
        }
        Event::Paste(_) if waiting_in_terminal(state) => false,
        // Pointer navigation is classified by the destination it actually
        // changes (tab/pane/mode/selection), not by the event kind. A click or
        // scroll inside the focused vacant pane must not revoke a pending
        // creation's captured epoch and strand its admitted bytes.
        Event::Mouse(_) => false,
        _ => true,
    }
}

pub(crate) fn waiting_in_terminal(state: &State) -> bool {
    state.mode == Mode::Terminal
        && (state.initial_control.as_ref().is_some_and(|intent| {
            Some(intent.address) == state.focused_terminal() && intent.validity.is_current()
        }) || state.creations.pending.values().any(|pending| {
            pending.expected.is_none() && crate::creation_presentation::focused(state, pending)
        }))
}

/// Keyboard bytes belong to the focused terminal while the TUI is establishing
/// writer authority for it. That window is the same for a new creation, a
/// startup attach and an ordinary reattach: the bytes are buffered against the
/// exact connection until the grant arrives, instead of being dropped.
pub(crate) fn admitted_pregrant(state: &State, address: TerminalAddress) -> bool {
    state.mode == Mode::Terminal
        && state.focused_terminal() == Some(address)
        // An interrupted stream requires explicit recovery before new bytes may
        // be associated with a fresh writer. Output loss is not a place to
        // accumulate keyboard input either: the operator cannot see what the
        // terminal received. A transient resync/geometry window, by contrast,
        // keeps the same visible surface; those bytes are retained by the
        // bounded pregrant queue until readiness returns.
        && !crate::terminal_input_authority::blocked(state, address)
        && !state.is_offline(address)
        && (matches!(
            crate::terminal_lifecycle::phase(state, address),
            yolo_tui_lifecycle::TerminalPhase::Starting
                | yolo_tui_lifecycle::TerminalPhase::Unavailable
        ) || state
            .initial_control
            .as_ref()
            .is_some_and(|intent| intent.address == address && intent.validity.is_current())
            || awaiting_writer(state, address))
}

fn awaiting_writer(state: &State, address: TerminalAddress) -> bool {
    let Some(target) = state
        .terminals
        .get(&address)
        .and_then(|session| session.fields().runtime)
        .map(|runtime| runtime.target)
    else {
        return false;
    };
    state.control_generation.is_some() && state.control.acquiring(target, std::time::Instant::now())
}

/// Classify terminal payload events so a context change can retire deferred
/// bytes. The event loop still forwards every event to the action router.
pub(crate) fn is_terminal_payload(state: &State, event: &Event) -> bool {
    if state.mode != Mode::Terminal {
        return false;
    }
    match event {
        Event::Paste(_) => true,
        Event::Key(key) if key.kind == KeyEventKind::Release => false,
        Event::Key(key) if state.prefix => false,
        Event::Key(key)
            if key.code == KeyCode::F(1)
                || (key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL)
                || (key.code == KeyCode::Esc && state.selection.active.is_some()) =>
        {
            false
        }
        Event::Key(_) => true,
        Event::Mouse(mouse) => crate::pointer_input::forwards(state, *mouse),
        Event::Resize(_, _) | Event::FocusGained | Event::FocusLost => false,
    }
}
