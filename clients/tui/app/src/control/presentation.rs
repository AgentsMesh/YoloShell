use crate::{State, control::OwnerClass};
use std::time::Instant;
use yolo_hub_identity::TerminalAddress;
use yolo_tui_inventory::ControlBadge;
use yolo_tui_terminal_control::Presentation;

const RESTORING: &str = "Restoring input";
const INPUT_PAUSED: &str = "Input paused";
const INPUT_RESUMING: &str = "Resuming input";
const PREVIOUS_LEASE: &str = "Resuming input · waiting for previous lease";
const OCCUPIED: &str = "Input control occupied";
const DENIED: &str = "Input access denied";
const HELD_BY_OTHER: &str = "Read Only · owner unknown";
const HELD_BY: &str = "Read Only · held by ";
const HELD_BY_CLIENT: &str = "Read Only · held by another Yolo UI";
const HELD_BY_THIS_CONTROLLER: &str = "Read Only · held by this UI instance";
const THIS_CLIENT: &str = "Control: this UI";
const OBSERVE_ONLY: &str = "Read Only · Ctrl-b t to take control";
const OBSERVED_HOLDER_MAX_BYTES: usize = 12;

pub(crate) fn routine(message: &str) -> bool {
    [RESTORING, OCCUPIED, DENIED].contains(&message)
}

pub(crate) fn label(state: &State, address: TerminalAddress) -> &'static str {
    match phase(state, address) {
        Some(Presentation::Acquiring | Presentation::Recovering) => RESTORING,
        Some(Presentation::Occupied) => OCCUPIED,
        Some(Presentation::Denied) => DENIED,
        _ => crate::constants::READ_ONLY_LABEL,
    }
}

pub(crate) fn phase(state: &State, address: TerminalAddress) -> Option<Presentation> {
    let now = Instant::now();
    state
        .control
        .entries
        .get(&address)
        .map(|entry| entry.policy.phase(now, state.grant(address, now).is_some()))
}

pub(crate) fn detail(state: &State, address: TerminalAddress) -> String {
    if !state
        .client_capability_enabled(yolo_builtin_client_contract::ClientCapability::TerminalControl)
    {
        return crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE.into();
    }
    if crate::terminal_input_authority::blocked(state, address) {
        return INPUT_PAUSED.to_owned();
    }
    if initial_control_pending(state, address) {
        return yolo_tui_render::SYNCING_LABEL.to_owned();
    }
    let now = Instant::now();
    let has_grant = state.grant(address, now).is_some();
    let phase = state
        .control
        .entries
        .get(&address)
        .map(|entry| entry.policy.phase(now, has_grant));
    if has_grant {
        return THIS_CLIENT.to_owned();
    }
    if matches!(phase, Some(Presentation::Denied)) {
        return DENIED.to_owned();
    }
    if let Some(owner) = state.control.observed_owner(address, state.principal) {
        return match owner {
            OwnerClass::ThisController => {
                if state.inputs.interrupted.contains_key(&address) {
                    return PREVIOUS_LEASE.to_owned();
                }
                format!("{HELD_BY_THIS_CONTROLLER} · Ctrl-b t to take control")
            }
            OwnerClass::SameClient => format!("{HELD_BY_CLIENT} · Ctrl-b t to take control"),
            OwnerClass::Other => state.control.observed_holder(address).map_or_else(
                || format!("{HELD_BY_OTHER} · Ctrl-b t to take control"),
                |holder| {
                    format!(
                        "{HELD_BY}{} · Ctrl-b t to take control",
                        compact_holder(holder)
                    )
                },
            ),
        };
    }
    if matches!(
        phase,
        Some(Presentation::Acquiring | Presentation::Recovering)
    ) {
        if state.inputs.interrupted.contains_key(&address) {
            return INPUT_RESUMING.to_owned();
        }
        return label(state, address).to_owned();
    }
    if matches!(phase, Some(Presentation::Occupied)) {
        return format!("{HELD_BY_OTHER} · Ctrl-b t to take control");
    }
    if matches!(phase, Some(Presentation::Denied)) {
        return label(state, address).to_owned();
    }
    OBSERVE_ONLY.to_owned()
}

pub(crate) fn badge(state: &State, address: TerminalAddress) -> ControlBadge {
    if crate::terminal_input_authority::blocked(state, address) {
        return ControlBadge::Paused;
    }
    if initial_control_pending(state, address) {
        return ControlBadge::Syncing;
    }
    if !state.control.entries.contains_key(&address) && !state.terminals.contains_key(&address) {
        return ControlBadge::Unavailable;
    }
    if state.grant(address, Instant::now()).is_some() {
        ControlBadge::Writable
    } else {
        ControlBadge::ReadOnly
    }
}

pub(crate) fn marker(state: &State, address: TerminalAddress) -> Option<&'static str> {
    if matches!(
        crate::terminal_lifecycle::phase(state, address),
        yolo_tui_lifecycle::TerminalPhase::Ended
            | yolo_tui_lifecycle::TerminalPhase::Lost
            | yolo_tui_lifecycle::TerminalPhase::Archived
    ) {
        return None;
    }
    if !state.control.entries.contains_key(&address) && !state.terminals.contains_key(&address) {
        return None;
    }
    Some(badge(state, address).glyph())
}

fn initial_control_pending(state: &State, address: TerminalAddress) -> bool {
    state
        .initial_control
        .as_ref()
        .is_some_and(|intent| intent.address == address && intent.validity.is_current())
}

fn compact_holder(value: &str) -> &str {
    let end = value
        .char_indices()
        .take_while(|(index, _)| *index < OBSERVED_HOLDER_MAX_BYTES)
        .map(|(index, character)| index + character.len_utf8())
        .last()
        .unwrap_or_default();
    &value[..end.min(value.len())]
}
