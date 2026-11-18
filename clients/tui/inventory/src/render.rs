use crate::{Inventory, InventoryViewport, View, constants::*, rows, sections};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{List, ListState, Paragraph},
};
use std::collections::BTreeMap;
use yolo_hub_identity::TerminalAddress;
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
    area: Rect,
) -> InventoryViewport {
    let bands = Layout::vertical([
        Constraint::Length(SECTION_ROWS),
        Constraint::Length(CONTEXT_ROWS),
        Constraint::Min(0),
    ])
    .split(area);
    let tabs = sections::render(frame, inventory.view, bands[0]);
    let mut viewport = InventoryViewport::new(
        area,
        bands[2],
        tabs,
        inventory,
        terminals,
        session_selected,
        filter,
    );
    let scope = inventory
        .scope
        .filter(|_| inventory.view != View::Hubs)
        .map_or_else(|| ALL_HUBS.to_owned(), |hub| format!("{HUB_LABEL} {hub}"));
    let context = format!(
        "{} | {scope} | {}",
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
        bands[1],
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
                        bands[2].width,
                    )
                })
                .collect::<Vec<_>>();
            (items, selected, EMPTY_SESSIONS)
        }
        View::Agents => {
            if !inventory.agents_supported {
                empty(frame, AGENTS_UNSUPPORTED, bands[2]);
                return viewport;
            }
            let visible = inventory.visible_agents(filter, terminals);
            let selected = inventory
                .selected_agent
                .and_then(|selected| visible.iter().position(|address| *address == selected));
            let items = visible
                .iter()
                .filter_map(|target| inventory.agents.get(target))
                .map(|agent| {
                    rows::agent(
                        agent,
                        terminals,
                        inventory.current,
                        inventory
                            .attention
                            .contains(&agent.fields().terminal.address),
                        control_status
                            .get(&agent.fields().terminal.address)
                            .copied(),
                        animation_phase,
                        bands[2].width,
                    )
                })
                .collect();
            (items, selected, EMPTY_AGENTS)
        }
        View::Hubs => {
            if !inventory.hubs_supported {
                empty(frame, HUBS_UNSUPPORTED, bands[2]);
                return viewport;
            }
            let visible = inventory.visible_hubs(filter);
            let selected = inventory
                .selected_hub
                .and_then(|selected| visible.iter().position(|hub| *hub == selected));
            let items = visible
                .iter()
                .filter_map(|hub| inventory.hubs.get(hub))
                .map(|hub| {
                    rows::hub(
                        hub,
                        inventory.current,
                        inventory.local_hub_name(hub.hub_id),
                        bands[2].width,
                    )
                })
                .collect();
            (items, selected, EMPTY_HUBS)
        }
    };
    if items.is_empty() {
        self::empty(frame, empty, bands[2]);
        return viewport;
    }
    let mut selection = ListState::default().with_selected(selected);
    let heights = items.iter().map(|item| item.height()).collect::<Vec<_>>();
    let mut list = List::new(items).highlight_style(Style::default().fg(SELECTED_COLOR));
    let marker = Line::from(SELECTION_MARKER);
    // Ratatui sizes the marker independently of the list rectangle.
    if marker.width() <= usize::from(bands[2].width) {
        list = list.highlight_symbol(marker);
    }
    frame.render_stateful_widget(list, bands[2], &mut selection);
    viewport.bind_rows(selection.offset(), &heights, inventory, terminals);
    viewport
}

fn empty(frame: &mut Frame<'_>, label: &'static str, area: Rect) {
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(MUTED_COLOR)),
        area,
    );
}
