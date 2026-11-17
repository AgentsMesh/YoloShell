use crate::{
    Mode, State,
    events::enqueue,
    form_actions::entry,
    network::Job,
    terminal_actions::{deliver, prefix},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;

include!("actions_confirmation.rs");

pub fn handle(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId, event: Event) -> bool {
    crate::startup::user_event(state, &event);
    if let Some(detach) = crate::modal_shortcuts::handle(state, &event) {
        return detach;
    }
    if state.tab_manager.is_some() {
        crate::tabs::manager::handle(state, jobs, hub, event);
        return false;
    }
    let modal = crate::help::handle(state, &event)
        || crate::workspace_ui::handle(state, jobs, hub, &event)
        || crate::document_view::handle(state, &event);
    if modal {
        return false;
    }
    if state.mode == Mode::HubConfiguration {
        crate::hub_actions::handle(state, jobs, event);
        return false;
    }
    // An explicit attach/new launched this client for a terminal; bytes typed
    // during its handshake belong to that terminal, not to the picker or
    // inventory that may briefly precede it.
    if let Some(data) = crate::startup::terminal_payload(state, &event) {
        if crate::terminal_input::enqueue_startup_pregrant(state, data) {
            state.status = crate::constants::INPUT_WAITING_LABEL.into();
        }
        return false;
    }
    let payload_event = crate::input_intent::is_terminal_payload(state, &event);
    if crate::input_intent::changes_context(state, &event) {
        let prefix_key = matches!(
            event,
            Event::Key(key)
                if key.code == KeyCode::Char('b')
                    && key.modifiers == KeyModifiers::CONTROL
        );
        let resume_key = state.prefix
            && matches!(
                event,
                Event::Key(key)
                    if key.kind == KeyEventKind::Press
                        && (key.code == KeyCode::Char(crate::help_constants::ACQUIRE_KEY)
                            || key.code == KeyCode::Char(crate::help_constants::TAKEOVER_KEY))
            );
        if !payload_event && !prefix_key && !resume_key {
            crate::terminal_input::clear_deferred(state);
        }
        // A payload admitted for an exact initial-control intent belongs to
        // that terminal even if the event classifier conservatively reports
        // a context change. Keep the intent and its pregrant queue together;
        // actual navigation still invalidates it through user_event.
        if !(payload_event && state.initial_control.is_some()) {
            crate::initial_control::user_event(state);
        }
    }
    if crate::hub_actions::open_event(state, &event) {
        crate::hub_actions::open(state, jobs, hub);
        return false;
    }
    if matches!(event, Event::Paste(_)) && crate::chat::handle(state, jobs, &event) {
        return false;
    }
    if crate::form_input::edit(state, &event) {
        return false;
    }
    if let Event::Mouse(mouse) = event {
        crate::pointer::handle(state, jobs, mouse);
        return false;
    }
    if matches!(event, Event::Resize(_, _) | Event::FocusLost) {
        crate::resize::invalidate_layout(state, matches!(event, Event::Resize(_, _)));
    }
    if let Event::Paste(text) = event {
        crate::terminal_actions::paste(state, jobs, &text);
        return false;
    }
    let Event::Key(key) = event else {
        return false;
    };
    if key.kind == KeyEventKind::Release {
        return false;
    }
    if key.kind == KeyEventKind::Press
        && (key.code == KeyCode::F(1)
            || (state.mode == Mode::Inventory
                && key.code == KeyCode::Char(crate::help_constants::HELP_KEY)))
    {
        crate::help::open(state);
        return false;
    }
    if matches!(state.mode, Mode::Terminal | Mode::Inventory | Mode::Chat) {
        if key.code == KeyCode::Esc && state.selection.active.is_some() {
            crate::selection_actions::cancel(state);
            state.prefix = false;
            return false;
        }
        if state.prefix {
            if key.kind != KeyEventKind::Press {
                return false;
            }
            state.prefix = false;
            if key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL {
                deliver(state, jobs, Ok(vec![crate::tabs::LITERAL_PREFIX]));
                return false;
            }
            return prefix(state, jobs, hub, key.code);
        }
        if key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL {
            state.prefix = true;
            return false;
        }
    }
    if crate::chat::handle(state, jobs, &Event::Key(key)) {
        return false;
    }
    crate::selection_actions::cancel(state);
    if state.mode == Mode::Terminal {
        let encoded = match state.focused_terminal() {
            Some(address) => state
                .surfaces
                .get(&address)
                .map(|surface| yolo_tui_input::encode_key(key, surface.screen()))
                .or_else(|| {
                    crate::input_intent::admitted_pregrant(state, address)
                        .then(|| yolo_tui_input::encode_key_default(key))
                }),
            None => crate::input_intent::waiting_in_terminal(state)
                .then(|| yolo_tui_input::encode_key_default(key)),
        };
        if let Some(encoded) = encoded {
            deliver(state, jobs, encoded);
        } else {
            state.status = if crate::input_intent::waiting_in_terminal(state)
                || state
                    .focused_terminal()
                    .is_some_and(|address| crate::input_intent::admitted_pregrant(state, address))
            {
                crate::constants::INPUT_WAITING_LABEL
            } else {
                yolo_tui_render::SYNCING_LABEL
            }
            .into();
        }
        return false;
    }
    if matches!(state.mode, Mode::Search | Mode::Directory) {
        entry(state, jobs, hub, key.code);
        return false;
    }
    if matches!(
        state.mode,
        Mode::ConfirmTerminate | Mode::ConfirmArchive | Mode::ConfirmRecreate
    ) {
        handle_confirmation(state, jobs, key);
        return false;
    }
    if key.code == KeyCode::Char(crate::help_constants::NEW_SHELL_KEY)
        && key.modifiers.is_empty()
        && key.kind == KeyEventKind::Press
    {
        crate::shell_actions::current_directory(state, jobs, hub);
        return false;
    }
    if key.code == KeyCode::Char('n') && key.modifiers.is_empty() {
        // The inventory handler owns the transition into the directory form;
        // this shortcut only prefills that form with the destination cwd.
        let origin = crate::shell_actions::destination(state, hub);
        let _ = crate::inventory_actions::handle(state, jobs, key.code);
        if state.mode == Mode::Directory {
            state.entry = crate::shell_actions::directory(state, origin);
        }
        return false;
    }
    if state.inventory.view == yolo_tui_inventory::View::Agents
        && key.code == KeyCode::Char(crate::interrupt_constants::KEY)
        && key.modifiers.is_empty()
    {
        if key.kind == KeyEventKind::Press {
            crate::interrupt_actions::start(state, jobs);
        }
        return false;
    }
    crate::inventory_actions::handle(state, jobs, key.code)
}
