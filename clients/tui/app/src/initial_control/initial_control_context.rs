use super::Intent;
use crate::State;
use std::time::Instant;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_identity::RuntimeTarget;

pub(super) fn current(state: &State, intent: &Intent) -> bool {
    state.client_capability_enabled(ClientCapability::TerminalControl)
        && state.control_generation == Some(intent.generation)
        && state.ui_epoch == intent.ui_epoch
        && state.tabs.active().id == intent.tab
        && state.tab_layout().focused() == intent.pane
        && state
            .tab_layout()
            .panes()
            .iter()
            .any(|pane| pane.id == intent.pane && pane.terminal == Some(intent.address))
}

pub(crate) fn valid_target(state: &State, intent: &Intent, target: RuntimeTarget) -> bool {
    current(state, intent)
        && intent.validity.is_current()
        && Instant::now() < intent.deadline
        && state.mode == crate::Mode::Terminal
        && intent.target == Some(target)
        && intent.origin == target.address.hub_id
        && intent.address == target.address
        && crate::observation_events::current(state, target)
}
