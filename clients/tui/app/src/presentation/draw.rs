use crate::presentation::Presentation;
use crate::{Mode, State, constants::*};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};
use std::collections::BTreeMap;
use yolo_tui_render::{TerminalPane, render_terminal};

pub fn draw(frame: &mut Frame<'_>, state: &State) -> Presentation {
    if state.help {
        let help = Some(crate::help::draw(frame, state.help_offset));
        return Presentation {
            help,
            ..Default::default()
        };
    }
    if state.workspaces.is_open() {
        yolo_tui_workspace::render(frame, &state.workspaces);
        return Presentation {
            mode: state.mode,
            ..Default::default()
        };
    }
    if let Some(manager) = &state.tab_manager {
        yolo_tui_tab_manager::render(frame, manager, &state.tab_snapshot(), frame.area());
        return Presentation::default();
    }
    if let Some(manager) = &state.hub_manager {
        let hub_view = Some(yolo_tui_hub_manager::render(frame, manager, frame.area()));
        return Presentation {
            mode: state.mode,
            hub_view,
            ..Default::default()
        };
    }
    let rows = Layout::vertical([
        Constraint::Length(crate::tabs::TAB_BAR_ROWS),
        Constraint::Min(0),
        Constraint::Length(crate::form_input::rows(state, frame.area().width)),
    ])
    .split(frame.area());
    let tab_strip = crate::tabs::strip::draw(frame, state, rows[0]);
    let tab_ids = state.tabs.entries().iter().map(|entry| entry.id).collect();
    let header_rows = crate::picker_header::height(state);
    let mut inventory_view = None;
    let mut hub_button = None;
    let bounds = if state.terminal_picker.is_some() && !state.document.is_visible(&state.tabs) {
        let picker =
            Layout::vertical([Constraint::Length(header_rows), Constraint::Min(0)]).split(rows[1]);
        if header_rows > 0 {
            crate::picker_header::draw(frame, state, picker[0]);
        }
        inventory_view = Some(inventory(frame, state, picker[1]));
        hub_button = inventory_view
            .as_ref()
            .and_then(|view| crate::hub_actions::draw_button(frame, state, view.area()));
        crate::status_bar::draw(frame, state, rows[2], None);
        return Presentation {
            bounds: rows[1],
            layout: Some(state.tab_layout().clone()),
            mode: state.mode,
            hub_button,
            inventory: inventory_view,
            tab_strip,
            tab_ids,
            zoomed: state.pane_view.zoomed,
            ..Presentation::default()
        };
    } else if (state.sidebar_visible || state.mode == Mode::Inventory)
        // Agent details render inside the content column. Keep the split even
        // when no terminal pane is bound, otherwise there is nowhere to draw
        // them (for example, a fresh `yolo terminals` layout).
        && (state.inventory.details.is_some()
            || state
                .tab_layout()
                .panes()
                .iter()
                .any(|pane| pane.terminal.is_some()))
        && rows[1].width >= crate::tabs::SIDEBAR_COLUMNS + crate::tabs::MIN_CONTENT_COLUMNS
    {
        let columns = Layout::horizontal([
            Constraint::Length(crate::tabs::SIDEBAR_COLUMNS),
            Constraint::Min(0),
        ])
        .split(rows[1]);
        let sidebar_area = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::RIGHT)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
        frame.render_widget(sidebar_area, columns[0]);
        let sidebar_bounds = Rect::new(
            columns[0].x,
            columns[0].y,
            columns[0]
                .width
                .saturating_sub(crate::tabs::SIDEBAR_DIVIDER_COLUMNS),
            columns[0].height,
        );
        let sidebar = Layout::vertical([Constraint::Length(header_rows), Constraint::Min(0)])
            .split(sidebar_bounds);
        if header_rows > 0 {
            crate::picker_header::draw(frame, state, sidebar[0]);
        }
        inventory_view = Some(inventory(frame, state, sidebar[1]));
        columns[1]
    } else if !matches!(state.mode, Mode::Terminal | Mode::Chat | Mode::Document)
        && !state.document.is_visible(&state.tabs)
    {
        let sidebar =
            Layout::vertical([Constraint::Length(header_rows), Constraint::Min(0)]).split(rows[1]);
        if header_rows > 0 {
            crate::picker_header::draw(frame, state, sidebar[0]);
        }
        inventory_view = Some(inventory(frame, state, sidebar[1]));
        Rect::default()
    } else {
        rows[1]
    };
    if let Some(view) = inventory_view.as_ref() {
        hub_button = crate::hub_actions::draw_button(frame, state, view.area());
    }
    if state.document.is_visible(&state.tabs) {
        state.document.render(frame, &state.tabs, bounds);
    }
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
    crate::status_bar::draw(frame, state, rows[2], desired);
    Presentation {
        bounds,
        panes: viewports,
        layout: Some(state.tab_layout().clone()),
        mode: state.mode,
        hub_button,
        hub_view: None,
        zoomed: state.pane_view.zoomed,
        inventory: inventory_view,
        tab_strip,
        tab_ids,
        help: None,
    }
}

include!("draw_inventory.rs");
