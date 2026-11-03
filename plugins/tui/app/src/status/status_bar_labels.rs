fn selected_title(state: &State) -> Option<&str> {
    match state.inventory.view {
        yolo_tui_inventory::View::Agents => state
            .inventory
            .details
            .or(state.inventory.selected_agent)
            .and_then(|address| state.inventory.agents.get(&address))
            .map(|agent| yolo_tui_inventory::agent_title(agent, &state.terminals)),
        yolo_tui_inventory::View::Terminals => state
            .selected
            .and_then(|address| state.terminals.get(&address))
            .map(|terminal| terminal.display_title()),
        yolo_tui_inventory::View::Hubs => None,
    }
}

fn important(state: &State) -> Option<String> {
    crate::terminal_input_authority::status(state)
        .map(str::to_owned)
        .or_else(|| crate::finalization_persistence::status(state))
        .or_else(|| crate::finalization_status::summary(state))
        .or_else(|| crate::creation_recovery::detail(state))
        .or_else(|| {
            state
                .notifications
                .message(std::time::Instant::now())
                .map(str::to_owned)
        })
        .or_else(|| {
            (state.mode == Mode::Terminal).then_some(())?;
            state
                .client_capability_enabled(ClientCapability::TerminalLifecycle)
                .then_some(())?;
            let address = state.focused_terminal()?;
            crate::terminal_lifecycle::projection(state, address, None)
                .action_hint()
                .map(str::to_owned)
        })
        .or_else(|| {
            let status = crate::interrupt_actions::status(state);
            let routine = [
                CONNECTED_LABEL,
                CONNECTING_LABEL,
                DISCONNECTED_LABEL,
                READ_ONLY_LABEL,
                yolo_tui_render::SYNCING_LABEL,
                yolo_tui_lifecycle::HUB_RECONNECTING_LABEL,
                yolo_tui_lifecycle::TERMINAL_RECONNECTING_LABEL,
                "",
            ];
            if routine.contains(&status.as_str())
                || crate::control::routine(&status)
                || (state.mode == Mode::Terminal
                    && status == crate::inventory_pointer::SELECTED_HINT)
            {
                None
            } else {
                Some(status)
            }
        })
}
