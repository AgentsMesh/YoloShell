pub(crate) fn flush_pregrant(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    address: TerminalAddress,
    grant: LeaseGrant,
) {
    // Keep the batch owned by its intent while the destination is blocked.
    // Taking the queue here and latching readiness would send bytes during
    // offline/resync without any visible screen.
    if state.input_blocked(address) {
        return;
    }
    let generation = state.control_generation;
    let focused = state.mode == crate::Mode::Terminal && state.focused_terminal() == Some(address);
    if !state.keyboard_ready(address) {
        state.inputs.snapshot_pending.insert(address);
    }
    let mut retained = VecDeque::new();
    let mut pending = std::mem::take(&mut state.inputs.pregrant);
    while let Some(item) = pending.pop_front() {
        if item.target != PregrantTarget::Address(address) {
            retained.push_back(item);
            continue;
        }
        // Bytes admitted for an exact acquisition episode do not survive a
        // connection change or navigation away from that terminal.
        if !focused || generation != item.generation {
            continue;
        }
        let data = item.data;
        let Ok(request) = InputRequest::new(yolo_hub_terminal_protocol::InputFields {
            request_id: Uuid::new_v4(),
            grant,
            data: data.clone(),
        }) else {
            state.status = crate::constants::UNEXPECTED_REPLY_LABEL.into();
            continue;
        };
        if !super::enqueue_current(state, jobs, request) {
            retained.push_back(PregrantInput {
                target: item.target,
                generation: item.generation,
                data,
            });
            retained.extend(pending);
            break;
        }
    }
    state.inputs.pregrant = retained;
}

/// Drop bytes whose intent no longer exists. Address items are settled by
/// their grant, but creation and startup intents can disappear silently.
pub(crate) fn prune_pregrant(state: &mut State) {
    let generation = state.control_generation;
    let creations = state
        .creations
        .pending
        .iter()
        .filter(|(_, pending)| {
            pending.expected.is_none()
                && (crate::creation_presentation::current(state, pending)
                    || crate::creation_presentation::input_focused(state, pending))
        })
        .map(|(id, _)| *id)
        .collect::<BTreeSet<_>>();
    let startup = crate::startup::terminal_intent_pending(state);
    state.inputs.pregrant.retain(|item| match item.target {
        PregrantTarget::Address(_) => true,
        PregrantTarget::Creation(id) => creations.contains(&id) && item.generation == generation,
        PregrantTarget::Startup => startup,
    });
}

fn current_creation(state: &State) -> Option<Uuid> {
    state.creations.pending.iter().find_map(|(id, pending)| {
        (pending.expected.is_none()
            && (crate::creation_presentation::current(state, pending)
                || crate::creation_presentation::input_focused(state, pending)))
        .then_some(*id)
    })
}

pub(crate) fn clear_startup_pregrant(state: &mut State) {
    state
        .inputs
        .pregrant
        .retain(|item| item.target != PregrantTarget::Startup);
}

pub(crate) fn clear_pregrant_for(state: &mut State, address: TerminalAddress) {
    state
        .inputs
        .pregrant
        .retain(|item| item.target != PregrantTarget::Address(address));
}

pub(crate) fn clear_pregrant(state: &mut State) {
    state.inputs.pregrant.clear();
}

/// Safety net for a retained batch whose destination became ready without a
/// new grant (for example a completed resync). The focused terminal is the only
/// destination a keyboard batch can still be addressed to.
pub(crate) fn drain_pregrant(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if state.mode != crate::Mode::Terminal {
        return;
    }
    let Some(address) = state.focused_terminal() else {
        return;
    };
    if state.input_blocked(address) {
        return;
    }
    let Some(grant) = state.grant(address, std::time::Instant::now()) else {
        return;
    };
    flush_pregrant(state, jobs, address, grant);
}
