use crate::presentation::Presentation;
use crate::{Mode, State};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};
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
        Constraint::Min(0),
        Constraint::Length(crate::form_input::rows(state, frame.area().width)),
    ])
    .split(frame.area());
    let header_rows = crate::picker_header::height(state);
    let mut inventory_view = None;
    let mut hub_button = None;
    if state.terminal_picker.is_some() && !state.document.is_visible(&state.tabs) {
        let body =
            Layout::vertical([Constraint::Length(crate::tabs::TAB_BAR_ROWS), Constraint::Min(0)])
                .split(rows[0]);
        let tab_strip = crate::tabs::strip::draw(frame, state, body[0]);
        let picker =
            Layout::vertical([Constraint::Length(header_rows), Constraint::Min(0)]).split(body[1]);
        if header_rows > 0 {
            crate::picker_header::draw(frame, state, picker[0]);
        }
        inventory_view = Some(inventory(frame, state, picker[1]));
        crate::status_bar::draw(frame, state, rows[1], None);
        return Presentation {
            bounds: body[1],
            layout: Some(state.tab_layout().clone()),
            mode: state.mode,
            hub_button,
            inventory: inventory_view,
            tab_strip,
            tab_ids: state.tabs.machine_entries().iter().map(|entry| entry.id).collect(),
            zoomed: state.pane_view.zoomed,
            ..Presentation::default()
        };
    }
    let show_sidebar = (state.sidebar_visible || state.mode == Mode::Inventory)
        && rows[0].width >= crate::tabs::SIDEBAR_COLUMNS + crate::tabs::MIN_CONTENT_COLUMNS;
    let columns = if show_sidebar {
        let columns = Layout::horizontal([
            Constraint::Length(crate::tabs::SIDEBAR_COLUMNS),
            Constraint::Min(0),
        ])
        .split(rows[0]);
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
        let sidebar = Layout::vertical([
            Constraint::Length(header_rows),
            Constraint::Min(0),
            Constraint::Length(crate::tabs::FOOTER_ROWS),
        ])
        .split(sidebar_bounds);
        if header_rows > 0 {
            crate::picker_header::draw(frame, state, sidebar[0]);
        }
        inventory_view = Some(inventory(frame, state, sidebar[1]));
        hub_button = crate::hub_actions::draw_footer(frame, state, sidebar[2]);
        columns[1]
    } else {
        rows[0]
    };
    let content =
        Layout::vertical([Constraint::Length(crate::tabs::TAB_BAR_ROWS), Constraint::Min(0)])
            .split(columns);
    let tab_strip = crate::tabs::strip::draw(frame, state, content[0]);
    let tab_ids = state.tabs.machine_entries().iter().map(|entry| entry.id).collect();
    let bounds = if inventory_view.is_some() {
        content[1]
    } else if !matches!(state.mode, Mode::Terminal | Mode::Chat | Mode::Document)
        && !state.document.is_visible(&state.tabs)
    {
        let sidebar =
            Layout::vertical([Constraint::Length(header_rows), Constraint::Min(0)]).split(content[1]);
        if header_rows > 0 {
            crate::picker_header::draw(frame, state, sidebar[0]);
        }
        inventory_view = Some(inventory(frame, state, sidebar[1]));
        Rect::default()
    } else {
        content[1]
    };
    if state.document.is_visible(&state.tabs) {
        state.document.render(frame, &state.tabs, bounds);
    }
    let (desired, viewports) = crate::draw_content::render(frame, state, bounds);
    crate::status_bar::draw(frame, state, rows[1], desired);
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
