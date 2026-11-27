use crate::{
    ControlBadge, Inventory, TreeRow, TreeRowKind,
    constants::*,
    rows,
};
use ratatui::{
    style::{Modifier, Style},
    text::Line,
    widgets::ListItem,
};
use std::collections::BTreeMap;
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;

pub(crate) fn item(
    inventory: &Inventory,
    row: TreeRow,
    terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    current: bool,
    control_status: &BTreeMap<TerminalAddress, ControlBadge>,
    animation_phase: u8,
    width: u16,
    active_machine: Option<HubId>,
) -> Option<ListItem<'static>> {
    match row.kind {
        TreeRowKind::Machine(hub) => Some(machine(
            inventory,
            hub,
            row.expanded,
            row.count,
            current,
            width,
            active_machine,
        )),
        TreeRowKind::Agent(target) => {
            let agent = inventory.agents.get(&target)?;
            let address = agent.fields().terminal.address;
            Some(rows::agent(
                agent,
                terminals,
                current,
                inventory.attention.contains(&address),
                control_status.get(&address).copied(),
                animation_phase,
                width,
                row.depth,
            ))
        }
    }
}

fn machine(
    inventory: &Inventory,
    hub: HubId,
    expanded: bool,
    count: usize,
    current: bool,
    width: u16,
    active_machine: Option<HubId>,
) -> ListItem<'static> {
    let reachable = inventory
        .machines
        .get(&hub)
        .is_some_and(|summary| summary.reachable);
    let (glyph, style) = if !current {
        (HUB_UNKNOWN_GLYPH, Style::default().fg(MUTED_COLOR))
    } else if reachable {
        (HUB_REACHABLE_GLYPH, Style::default())
    } else {
        (HUB_UNREACHABLE_GLYPH, Style::default().fg(WARNING_COLOR))
    };
    let expander = if count == 0 {
        "  "
    } else if expanded {
        "▾ "
    } else {
        "▸ "
    };
    let mut text = format!("{expander}{}", inventory.machine_label(hub));
    if count > 0 {
        text.push_str(&format!(" ({count})"));
    }
    text.push_str(&format!(" {glyph}"));
    let style = if active_machine == Some(hub) {
        Style::default()
            .fg(SELECTED_COLOR)
            .add_modifier(Modifier::BOLD)
    } else {
        style
    };
    let text = elide(&text, width);
    ListItem::new(Line::from(text).style(style))
}

fn elide(value: &str, width: u16) -> String {
    let width = usize::from(width);
    if Line::raw(value).width() <= width {
        return value.to_owned();
    }
    let mut result = String::new();
    let mut used = 0;
    for character in value.chars() {
        let size = Line::raw(character.to_string()).width();
        if used + size + 1 > width {
            break;
        }
        result.push(character);
        used += size;
    }
    result.push('…');
    result
}
