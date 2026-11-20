fn inventory(
    frame: &mut Frame<'_>,
    state: &State,
    area: Rect,
) -> yolo_tui_inventory::InventoryViewport {
    let control_status = state
        .terminals
        .keys()
        .map(|address| (*address, crate::control::badge(state, *address)))
        .collect();
    yolo_tui_inventory::render_viewport(
        frame,
        &state.inventory,
        &state.terminals,
        state.selected,
        &state.filter,
        &control_status,
        animation_phase(),
        Some(state.tabs.machine()),
        area,
    )
}

fn animation_phase() -> u8 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    ((millis / u128::from(yolo_tui_inventory::SPINNER_FRAME_MS)) % 10) as u8
}
