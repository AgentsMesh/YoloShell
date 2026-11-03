use crate::{State, network::Job, startup_constants::*};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use tokio::sync::mpsc;
use yolo_hub_identity::{HubId, TerminalAddress};

/// A single user entry intent. It never grants permission to replay an uncertain mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StartupAction {
    Automatic { cwd: String },
    NewShell { cwd: String, name: Option<String> },
    Attach { selector: Option<String> },
    Inventory,
    TerminalPicker,
}

#[derive(Default)]
pub(crate) struct Entry {
    pub hub: Option<HubId>,
    pub directory: Option<String>,
    pending: Option<StartupAction>,
    /// The restored focus before terminal liveness can retire its tab. Startup
    /// keeps using it to route a dead saved terminal to history instead of
    /// silently attaching or creating something else.
    saved: Option<TerminalAddress>,
    /// An explicit attach retries against catalogue pages until this deadline.
    pub(crate) attach_deadline: Option<std::time::Instant>,
}

pub(crate) fn install(state: &mut State, hub: HubId, action: StartupAction) {
    if matches!(
        action,
        StartupAction::Inventory | StartupAction::TerminalPicker
    ) && !state.document.is_visible(&state.tabs)
    {
        state.mode = crate::Mode::Inventory;
    }
    if matches!(action, StartupAction::TerminalPicker) && !state.document.is_visible(&state.tabs) {
        crate::tabs::terminal_picker::open(state);
    }
    if matches!(action, StartupAction::Attach { .. }) {
        state.status = SEARCHING_TERMINAL.into();
        // Show the catalogue immediately instead of a blank inventory while
        // the explicit target is retried across catalogue pages. The target
        // is still attached automatically when it appears.
        crate::tabs::terminal_picker::open(state);
    }
    state.startup.hub = Some(hub);
    state.startup.directory = match &action {
        StartupAction::Automatic { cwd } | StartupAction::NewShell { cwd, .. } => Some(cwd.clone()),
        _ => Some(crate::constants::DEFAULT_DIRECTORY.into()),
    };
    state.startup.saved = state.focused_terminal();
    state.startup.attach_deadline = matches!(action, StartupAction::Attach { .. })
        .then(|| std::time::Instant::now() + STARTUP_SELECTION_TIMEOUT);
    state.startup.pending = (!matches!(action, StartupAction::Inventory)).then_some(action);
}

pub(crate) fn user_event(state: &mut State, event: &Event) {
    let navigation = match event {
        Event::Key(key) => key.kind == KeyEventKind::Press,
        Event::Paste(_) => true,
        Event::Mouse(mouse) => matches!(
            mouse.kind,
            MouseEventKind::Down(_) | MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ),
        _ => false,
    };
    // Automatic startup is a convenience that user navigation may replace.
    // Explicit `new` and `attach` commands are user intent and must survive a
    // slow Hub/catalogue handshake; cancelling them on the first key made the
    // command appear to do nothing.
    if navigation && matches!(state.startup.pending, Some(StartupAction::Automatic { .. })) {
        state.startup.pending = None;
    }
    if matches!(
        event,
        Event::Key(key)
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc
    ) && matches!(state.startup.pending, Some(StartupAction::TerminalPicker))
    {
        state.startup.pending = None;
    }
}

/// Bytes typed before an explicit `attach`/`new` entry reaches its terminal.
/// Navigation stays interactive; the payload set matches a bound terminal.
pub(crate) fn terminal_intent_pending(state: &State) -> bool {
    matches!(
        state.startup.pending,
        Some(StartupAction::Attach { .. } | StartupAction::NewShell { .. })
    )
}

pub(crate) fn terminal_payload(state: &State, event: &Event) -> Option<Vec<u8>> {
    // Pre-catalogue bytes belong to the pending terminal. Once a coherent
    // catalogue makes the inventory actionable, the inventory owns keyboard
    // navigation; capturing there would replay Enter/filter text into the
    // eventual PTY.
    if state.prefix
        || !terminal_intent_pending(state)
        || (state.mode == crate::Mode::Inventory && state.inventory.current)
    {
        return None;
    }
    match event {
        Event::Paste(text) => yolo_tui_input::encode_paste(text, false).ok(),
        Event::Key(key)
            if key.kind == KeyEventKind::Press
                && key.code != KeyCode::F(1)
                && !(key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL)
                && !(key.code == KeyCode::Esc && state.selection.active.is_some()) =>
        {
            yolo_tui_input::encode_key_default(*key).ok()
        }
        _ => None,
    }
}

include!("startup_apply.rs");
