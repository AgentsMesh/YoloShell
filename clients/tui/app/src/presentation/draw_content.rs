use crate::{Mode, State};
use ratatui::{Frame, layout::Rect};
use std::collections::BTreeMap;
use yolo_hub_terminal_protocol::TerminalSize;
use yolo_tui_render::{PaneViewport, TerminalPane, render_terminal};

pub(crate) fn render(
    frame: &mut Frame<'_>,
    state: &State,
    bounds: Rect,
) -> (Option<TerminalSize>, Vec<PaneViewport>) {
    let desired = state.focused_terminal_size(bounds);
    let terminals = state
        .terminals
        .iter()
        .map(|(address, session)| {
            (
                *address,
                TerminalPane {
                    screen: crate::selection_actions::display_screen(state, *address)
                        .or_else(|| state.surfaces.get(address).map(|surface| surface.screen())),
                    projection: crate::terminal_lifecycle::projection(state, *address, desired),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let viewports = if state.document.is_visible(&state.tabs)
        || bounds.is_empty()
        || (!matches!(state.mode, Mode::Terminal | Mode::Chat)
            && yolo_tui_inventory::render_details(
                frame,
                &state.inventory,
                &state.terminals,
                state
                    .inventory
                    .details
                    .and_then(|address| state.inventory.agents.get(&address))
                    .map(|agent| agent.fields().terminal.address)
                    .map(|address| crate::control::detail(state, address))
                    .as_deref(),
                bounds,
            )) {
        Vec::new()
    } else {
        render_terminal(
            frame,
            &state.display_layout(),
            &terminals,
            bounds,
            (state.mode == Mode::Chat).then_some(state.tab_layout().focused()),
        )
        .unwrap_or_default()
    };
    if state.mode == Mode::Chat {
        if let Some(chat) = crate::chat::current(state) {
            if let Some(view) = viewports
                .iter()
                .find(|view| view.pane == state.tab_layout().focused())
            {
                yolo_tui_chat::render(frame, chat, view.inner, crate::chat::input_readiness(state));
            }
        }
    }
    for view in &viewports {
        if let Some(selection) = crate::selection_actions::highlight(state, *view) {
            yolo_tui_render::highlight_selection(frame, *view, selection);
        }
    }
    (desired, viewports)
}
