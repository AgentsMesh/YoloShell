use crate::{Inventory, InventoryViewport, TreeSelection, View, constants::*, rows, tree_render};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{List, ListState, Paragraph},
};
use std::collections::BTreeMap;
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;

use crate::labels::ControlBadge;

pub fn render_viewport(
    frame: &mut Frame<'_>,
    inventory: &Inventory,
    terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    session_selected: Option<TerminalAddress>,
    filter: &str,
    control_status: &BTreeMap<TerminalAddress, ControlBadge>,
    animation_phase: u8,
    active_machine: Option<HubId>,
    area: Rect,
) -> InventoryViewport {
    let bands = Layout::vertical([Constraint::Length(CONTEXT_ROWS), Constraint::Min(0)]).split(area);
    let mut viewport = InventoryViewport::new(
        area,
        bands[1],
        inventory,
        terminals,
        session_selected,
        filter,
    );
    let machine = active_machine
        .map_or_else(|| ALL_MACHINES.to_owned(), |hub| inventory.machine_label(hub));
    let machine = match inventory.view {
        View::Terminals => format!("{TERMINAL_PICKER_LABEL} | {machine}"),
        View::Tree => machine,
    };
    let context = format!(
        "{} | {machine} | {}",
        if inventory.current {
            LIVE_LABEL
        } else {
            STALE_LABEL
        },
        if inventory.history {
            HISTORY_SCOPE
        } else {
            ACTIVE_SCOPE
        },
    );
    frame.render_widget(
        Paragraph::new(context).style(Style::default().fg(if inventory.current {
            MUTED_COLOR
        } else {
            WARNING_COLOR
        })),
        bands[0],
    );
    let (items, selected, empty) = match inventory.view {
        View::Terminals => {
            let visible = inventory.visible_terminals(filter, terminals);
            let selected = session_selected
                .and_then(|selected| visible.iter().position(|address| *address == selected));
            let items = visible
                .iter()
                .filter_map(|address| terminals.get(address).map(|session| (*address, session)))
                .map(|(address, session)| {
                    rows::terminal(
                        session,
                        inventory.phase(address, session.fields()),
                        inventory.attention.contains(&session.fields().address),
                        bands[1].width,
                    )
                })
                .collect::<Vec<_>>();
            (items, selected, EMPTY_SESSIONS)
        }
        View::Tree => {
            let rows = inventory.tree_rows(filter, terminals);
            if rows.is_empty() {
                let label = if inventory.machines_supported {
                    EMPTY_MACHINES
                } else {
                    MACHINES_UNSUPPORTED
                };
                empty(frame, label, bands[1]);
                return viewport;
            }
            let selected = inventory.tree_selected.and_then(|selected| {
                rows.iter().position(|row| {
                    matches!(
                        (row.kind, selected),
                        (crate::TreeRowKind::Machine(hub), TreeSelection::Machine(machine)) if hub == machine
                    ) || matches!(
                        (row.kind, selected),
                        (crate::TreeRowKind::Agent(target), TreeSelection::Agent(agent)) if target == agent
                    )
                })
            });
            let items = rows
                .into_iter()
                .filter_map(|row| {
                    tree_render::item(
                        inventory,
                        row,
                        terminals,
                        inventory.current,
                        control_status,
                        animation_phase,
                        bands[1].width,
                        active_machine,
                    )
                })
                .collect::<Vec<_>>();
            (items, selected, EMPTY_MACHINES)
        }
    };
    if items.is_empty() {
        self::empty(frame, empty, bands[1]);
        return viewport;
    }
    let mut selection = ListState::default().with_selected(selected);
    let heights = items.iter().map(|item| item.height()).collect::<Vec<_>>();
    let mut list = List::new(items).highlight_style(Style::default().fg(SELECTED_COLOR));
    let marker = Line::from(SELECTION_MARKER);
    // Ratatui sizes the marker independently of the list rectangle.
    if marker.width() <= usize::from(bands[1].width) {
        list = list.highlight_symbol(marker);
    }
    frame.render_stateful_widget(list, bands[1], &mut selection);
    viewport.bind_rows(selection.offset(), &heights, inventory, terminals);
    viewport
}

fn empty(frame: &mut Frame<'_>, label: &'static str, area: Rect) {
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(MUTED_COLOR)),
        area,
    );
}
