use crate::State;

pub(crate) const BLOCKED_LABEL: &str = "Shell confirmations blocked";

pub(crate) fn blocked(state: &State) -> usize {
    state
        .creations
        .finalizing
        .values()
        .filter(|entry| entry.rejection.is_some())
        .count()
        + state.creations.recovery.status().blocked
}

pub(crate) fn detail(state: &State) -> Option<String> {
    if !state.connection_ready() {
        return None;
    }
    state.creations.finalizing.values().find_map(|entry| {
        let error = entry.rejection.as_ref()?;
        let address = entry.record.creation.address;
        Some(format!(
            "{BLOCKED_LABEL}: Hub {} / terminal {}: {error}",
            address.hub_id,
            address.terminal_id.as_uuid()
        ))
    })
}

pub(crate) fn summary(state: &State) -> Option<String> {
    if !state.creations.finalizing.is_empty()
        && !state.client_capability_enabled(
            yolo_builtin_client_contract::ClientCapability::TerminalLifecycle,
        )
    {
        return Some(crate::client_capabilities::TERMINAL_LIFECYCLE_UNAVAILABLE.into());
    }
    if !state.connection_ready() {
        return None;
    }
    state.creations.finalizing.values().find_map(|entry| {
        let error = entry.rejection.as_ref()?;
        Some(format!("{BLOCKED_LABEL}: {} | {error}", blocked(state)))
    })
}
