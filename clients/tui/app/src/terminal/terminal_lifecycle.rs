use crate::State;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::TerminalSize;
use yolo_tui_lifecycle::{ConnectionState, InputAccess, TerminalPhase, TerminalProjection};

pub(crate) fn phase(state: &State, address: TerminalAddress) -> TerminalPhase {
    state
        .terminals
        .get(&address)
        .map_or(TerminalPhase::Unavailable, |terminal| {
            yolo_tui_lifecycle::terminal_phase(terminal.fields(), state.ended_epoch(address))
        })
}

pub(crate) fn input_blocked_hint(state: &State, address: TerminalAddress) -> Option<&'static str> {
    phase(state, address).input_blocked_hint()
}

/// A terminal that cannot become interactive again in this runtime epoch.
///
/// The Hub keeps the record for history and recovery, but a live presentation
/// must not bind it. A missing catalogue entry stays unjudged: partial pages
/// are not evidence that the terminal is gone.
pub(crate) fn gone(state: &State, address: TerminalAddress) -> bool {
    state.terminals.contains_key(&address)
        && matches!(
            phase(state, address),
            TerminalPhase::Ended | TerminalPhase::Lost | TerminalPhase::Archived
        )
}

pub(crate) fn projection(
    state: &State,
    address: TerminalAddress,
    desired: Option<TerminalSize>,
) -> TerminalProjection {
    let phase = phase(state, address);
    let connection = if state.control_generation.is_none() {
        ConnectionState::HubReconnecting
    } else if state.is_offline(address)
        && matches!(
            phase,
            TerminalPhase::Running | TerminalPhase::Starting | TerminalPhase::Finalizing
        )
    {
        ConnectionState::TerminalReconnecting
    } else {
        ConnectionState::Connected
    };
    let mut projection = TerminalProjection::new(
        phase,
        connection,
        state.grant(address, std::time::Instant::now()).is_some(),
        state.presentation_ready(address, state.focused_terminal(), desired),
    );
    // Initial attach/new-shell control is an in-flight write intent. It has no
    // writer grant yet, but presenting it as Read Only invites an unnecessary
    // takeover and misrepresents a terminal that this UI is already opening.
    if projection.input == InputAccess::ReadOnly
        && state
            .initial_control
            .as_ref()
            .is_some_and(|intent| intent.address == address && intent.validity.is_current())
    {
        projection.input = InputAccess::Synchronizing;
    }
    projection
}
