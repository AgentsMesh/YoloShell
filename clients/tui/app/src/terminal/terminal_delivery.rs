/// Plain-text paste is terminal input, not a picker edit. The focused
/// surface's bracketed-paste mode decides the envelope so a multi-line paste
/// is not interpreted as individual keystrokes.
pub(crate) fn paste(state: &mut State, jobs: &mpsc::Sender<Job>, text: &str) {
    if state.mode != crate::Mode::Terminal {
        return;
    }
    crate::selection_actions::cancel(state);
    let bracketed = state
        .focused_terminal()
        .and_then(|address| state.surfaces.get(&address))
        .is_some_and(|surface| surface.screen().bracketed_paste());
    deliver(state, jobs, yolo_tui_input::encode_paste(text, bracketed));
}

pub(crate) fn deliver(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    encoded: Result<Vec<u8>, yolo_tui_input::InputError>,
) -> bool {
    if state.mode == crate::Mode::Chat {
        return false;
    }
    let data = match encoded {
        Ok(data) => data,
        Err(error) => {
            state.status = error.to_string();
            return false;
        }
    };
    if data.is_empty() {
        return false;
    }
    if !crate::client_capabilities::require(
        state,
        yolo_builtin_client_contract::ClientCapability::TerminalControl,
        crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE,
    ) {
        return false;
    }
    let Some(address) = state.focused_terminal() else {
        // A focused vacant pane belongs to a pending creation whose address is
        // not known yet; its first grant releases these bytes.
        if crate::terminal_input::enqueue_creation_pregrant(state, data) {
            state.status = INPUT_WAITING_LABEL.into();
            return true;
        }
        return false;
    };
    if let Some(hint) = crate::terminal_lifecycle::input_blocked_hint(state, address) {
        // A creation/attach intent can render its first prompt before the
        // catalogue reports the terminal as running. Keep bytes with that
        // intent instead of rejecting this transient phase.
        if matches!(
            crate::terminal_lifecycle::phase(state, address),
            yolo_tui_lifecycle::TerminalPhase::Starting
                | yolo_tui_lifecycle::TerminalPhase::Unavailable
        ) && crate::input_intent::admitted_pregrant(state, address)
            && crate::terminal_input::enqueue_pregrant(state, address, data)
        {
            state.status = INPUT_WAITING_LABEL.into();
            return true;
        }
        state.status = hint.into();
        return false;
    }

    if crate::terminal_input_authority::blocked(state, address) {
        return false;
    }
    let Some(grant) = state.grant(address, std::time::Instant::now()) else {
        if crate::input_intent::admitted_pregrant(state, address)
            && crate::terminal_input::enqueue_pregrant(state, address, data)
        {
            state.status = INPUT_WAITING_LABEL.into();
            return true;
        }
        state.status = crate::control::label(state, address).into();
        return false;
    };
    if !state.keyboard_ready(address) {
        // A transient offline/resync window (or geometry without a current
        // surface) retains the bytes in the bounded pregrant queue; the run
        // loop drains them once the destination is ready. An interrupted
        // stream is handled separately above and requires explicit recovery.
        if state.input_blocked(address) {
            if state.is_offline(address) {
                state.status = yolo_tui_render::SYNCING_LABEL.into();
                return false;
            }
            if crate::terminal_input::enqueue_pregrant(state, address, data) {
                state.status = INPUT_WAITING_LABEL.into();
                return true;
            }
            state.status = yolo_tui_render::SYNCING_LABEL.into();
            return false;
        }
        state.inputs.snapshot_pending.insert(address);
    }
    if let Some(surface) = state.surfaces.get_mut(&address) {
        surface.set_scrollback(0);
    }
    match InputRequest::new(InputFields {
        request_id: Uuid::new_v4(),
        grant,
        data,
    }) {
        Ok(request) => crate::terminal_input::enqueue(state, jobs, request),
        Err(error) => {
            state.status = error.to_string();
            false
        }
    }
}
