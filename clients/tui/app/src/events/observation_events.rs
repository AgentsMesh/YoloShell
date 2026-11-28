use crate::State;
use yolo_hub_identity::{RuntimeTarget, TerminalAddress};
use yolo_hub_terminal_protocol::{AttachResult, PublicError, TerminalEvent};
use yolo_tui_terminal::TerminalSurface;

pub fn replace_surface(
    state: &mut State,
    address: TerminalAddress,
    attachment: &AttachResult,
) -> Result<(), PublicError> {
    let surface = TerminalSurface::from_result(attachment).ok_or(PublicError::OutputGap)?;
    state.surfaces.insert(address, surface);
    Ok(())
}

pub fn scoped(
    state: &mut State,
    jobs: &tokio::sync::mpsc::Sender<crate::network::Job>,
    target: RuntimeTarget,
    generation: uuid::Uuid,
    update: crate::network::ObservationUpdate,
) {
    use crate::network::{ObservationUpdate, Update};
    use yolo_hub_rpc_protocol::{Command, Reply};
    if state.observer_generation(target.address) != Some(generation) || !current(state, target) {
        return;
    }
    let update = match update {
        ObservationUpdate::Attached {
            request,
            attachment,
        } if request.target() == target => Update::Reply {
            command: Command::Attach { request },
            reply: Reply::Attached { attachment },
        },
        ObservationUpdate::Terminal { event } if event.target() == target => {
            if state.geometry_refresh_active(target.address)
                && matches!(
                    event,
                    TerminalEvent::Output { .. } | TerminalEvent::Heartbeat { .. }
                )
            {
                return;
            }
            crate::query_replies::live(state, jobs, generation, event);
            return;
        }
        ObservationUpdate::Closed { message } => {
            closed(state, target, message);
            return;
        }
        _ => return,
    };
    crate::events::apply(state, jobs, update);
}

pub fn terminal(state: &mut State, event: TerminalEvent) {
    let target = event.target();
    if !current(state, target) {
        return;
    }
    if state.geometry_refresh_active(target.address)
        && matches!(
            event,
            TerminalEvent::Output { .. } | TerminalEvent::Heartbeat { .. }
        )
    {
        return;
    }
    match &event {
        TerminalEvent::ResyncRequired { .. } => {
            state.surface_gap(target.address, None);
        }
        TerminalEvent::Unreachable { .. } => {
            state.clear_geometry_syncing(target.address);
            state.mark_offline(target.address);
        }
        _ => {
            if let Some(surface) = state.surfaces.get_mut(&target.address) {
                if let Err(error) = surface.apply_event(&event) {
                    state.surface_gap(target.address, Some(error.to_string()));
                }
            }
            if matches!(event, TerminalEvent::Exited { .. }) {
                state.end_runtime(target);
                if state.focused_terminal() == Some(target.address) {
                    state.status = yolo_tui_lifecycle::ENDED_HINT.into();
                }
                crate::tab_liveness::retire_dead(state);
            }
        }
    }
}

pub fn closed(state: &mut State, target: RuntimeTarget, message: Option<String>) {
    if !current(state, target) {
        return;
    }
    state.clear_geometry_syncing(target.address);
    if matches!(
        crate::terminal_lifecycle::phase(state, target.address),
        yolo_tui_lifecycle::TerminalPhase::Ended
            | yolo_tui_lifecycle::TerminalPhase::Lost
            | yolo_tui_lifecycle::TerminalPhase::Archived
            | yolo_tui_lifecycle::TerminalPhase::Finalizing
            | yolo_tui_lifecycle::TerminalPhase::OutputBlocked
    ) {
        state.clear_offline(target.address);
    } else {
        state.mark_offline(target.address);
    }
    // Output observation has no authority to revoke a healthy writer lease.
    // Readiness cancels pending input; lease recovery is managed separately.
    if state.focused_terminal() == Some(target.address) {
        state.status = crate::terminal_lifecycle::input_blocked_hint(state, target.address)
            .map(str::to_owned)
            .or(message)
            .unwrap_or_else(|| yolo_tui_lifecycle::TERMINAL_RECONNECTING_LABEL.into());
    }
}

pub fn current(state: &State, target: RuntimeTarget) -> bool {
    state
        .terminals
        .get(&target.address)
        .and_then(|session| session.fields().runtime)
        .is_some_and(|runtime| runtime.target == target)
}
