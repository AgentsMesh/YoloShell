pub(crate) fn apply(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if !state.inventory.current {
        return;
    }
    let Some(action) = state.startup.pending.take() else {
        return;
    };
    if matches!(action, StartupAction::TerminalPicker) {
        crate::tabs::terminal_picker::open(state);
        return;
    }
    if state.control_generation.is_none_or(|value| value.is_nil()) {
        state.startup.pending = Some(action);
        return;
    }
    let Some(hub) = state.startup.hub else { return };
    match action {
        StartupAction::Automatic { cwd } => automatic(state, jobs, hub, cwd),
        StartupAction::NewShell { cwd, name } => {
            if crate::shell_actions::create(state, jobs, hub, cwd, name) {
                crate::terminal_input::adopt_startup_creation(state);
            } else {
                crate::terminal_input::clear_startup_pregrant(state);
            }
        }
        StartupAction::Attach { selector } => {
            if selector.is_none()
                && let Some(address) = state.startup.saved
                && crate::terminal_lifecycle::gone(state, address)
            {
                state.startup.attach_deadline = None;
                crate::terminal_input::clear_startup_pregrant(state);
                retained(state, Some(address));
                return;
            }
            match crate::startup_selection::attach(state, hub, selector.as_deref()) {
                Ok(address) => {
                    state.startup.attach_deadline = None;
                    if crate::terminal_lifecycle::gone(state, address) {
                        crate::terminal_input::clear_startup_pregrant(state);
                        retained(state, Some(address));
                        return;
                    }
                    crate::tabs::terminal_picker::close(state);
                    crate::startup_attach::open(state, jobs, address);
                    crate::terminal_input::adopt_startup_pregrant(state, address);
                    if state.status == SEARCHING_TERMINAL {
                        state.status = crate::constants::CONNECTED_LABEL.into();
                    }
                    // A restored focus may already hold writer authority.
                    if let Some(grant) = state.grant(address, std::time::Instant::now()) {
                        crate::terminal_input::flush_pregrant(state, jobs, address, grant);
                    }
                }
                Err(message) => {
                    // The first coherent catalogue can precede the page that
                    // carries an explicit target. Keep the entry (and its
                    // admitted bytes) until the deadline instead of turning
                    // the next keystroke into inventory navigation.
                    if state
                        .startup
                        .attach_deadline
                        .is_some_and(|deadline| std::time::Instant::now() < deadline)
                    {
                        state.startup.pending = Some(StartupAction::Attach { selector });
                        return;
                    }
                    state.startup.attach_deadline = None;
                    crate::terminal_input::clear_startup_pregrant(state);
                    crate::tabs::terminal_picker::open(state);
                    state.status = message.into();
                }
            }
        }
        StartupAction::Inventory | StartupAction::TerminalPicker => unreachable!(),
    }
}

fn automatic(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId, cwd: String) {
    if let Some(address) = entry_terminal(state) {
        if state
            .terminals
            .get(&address)
            .is_some_and(crate::startup_selection::live)
        {
            crate::startup_attach::open(state, jobs, address);
        } else {
            retained(state, None);
        }
        return;
    }
    if let Some(address) = state
        .terminals
        .values()
        .find(|session| {
            session.fields().address.hub_id == hub && crate::startup_selection::live(session)
        })
        .map(|session| session.fields().address)
    {
        crate::startup_attach::open(state, jobs, address);
    } else if state.creations.len() != 0 {
        state.status = PENDING_CREATION.into();
    } else {
        crate::tabs::creation::create(
            state,
            jobs,
            hub,
            cwd,
            None,
            crate::tabs::creation::Placement::Empty,
        );
    }
}

/// The entry's restored terminal survives liveness retirement so startup can
/// route it to history instead of creating or attaching an unrelated shell.
fn entry_terminal(state: &State) -> Option<TerminalAddress> {
    state.focused_terminal().or(state
        .startup
        .saved
        .filter(|address| state.terminals.contains_key(address)))
}

/// Leaves a terminal that cannot accept input in the picker instead of opening
/// a dead pane. An explicitly targeted terminal is shown in history so its
/// recovery action is immediately reachable.
pub(crate) fn retained(state: &mut State, address: Option<TerminalAddress>) {
    crate::tabs::terminal_picker::open(state);
    if let Some(address) = address {
        state.inventory.history = true;
        state.selected = Some(address);
    }
    state.reconcile_selection();
    state.status = if address.is_some() {
        ENDED_SESSION.into()
    } else {
        RETAINED_SESSION.into()
    };
}
