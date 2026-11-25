use crate::{Inventory, constants::*, labels};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Paragraph, Wrap},
};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{Capability, Freshness, Reachability};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::TerminalSummary;

pub fn render_details(
    frame: &mut Frame<'_>,
    inventory: &Inventory,
    terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    control_status: Option<&str>,
    area: Rect,
) -> bool {
    let Some(address) = inventory.details else {
        return false;
    };
    let Some(agent) = inventory.agents.get(&address) else {
        return false;
    };
    let fields = agent.fields();
    let terminal = terminals.get(&fields.terminal.address);
    let directory = match &fields.working_directory {
        None => UNKNOWN_DIRECTORY_LABEL.to_owned(),
        Some(directory)
            if inventory.current
                && fields.freshness == Freshness::Current
                && fields.reachability == Reachability::Reachable =>
        {
            directory.as_str().to_owned()
        }
        Some(directory) => format!("{} ({LAST_KNOWN_DIRECTORY_LABEL})", directory.as_str()),
    };
    let provider = fields
        .provider
        .display_name()
        .unwrap_or_else(|| fields.provider.kind());
    let mut lines = vec![
        Line::from(AGENT_DETAILS_TITLE).style(Style::default().fg(SELECTED_COLOR)),
        Line::from(crate::agent_title(agent, terminals)),
        Line::from(format!("{provider} | {}", labels::lifecycle(fields.lifecycle))),
        Line::from(labels::activity(fields.activity)),
        control_status.map_or_else(|| Line::from(CONTROL_UNKNOWN_LABEL), Line::from),
        Line::from(format!(
            "{} | {} | {}",
            labels::source(fields.source),
            labels::observed_reachability(
                inventory.current,
                fields.reachability == Reachability::Reachable
            ),
            if inventory.current && fields.freshness == Freshness::Current {
                LIVE_LABEL
            } else {
                STALE_LABEL
            }
        )),
        Line::from(format!("{AGENT_LABEL}: {address}")),
        Line::from(format!(
            "{DETAILS_TERMINAL_LABEL}: {}",
            fields.terminal.address
        )),
        Line::from(format!("{DIRECTORY_LABEL}: {directory}")),
        Line::from(labels::recovery(fields.recovery)),
        Line::from(terminal.map_or(DETAILS_NO_TERMINAL, |_| DETAILS_RECOVERY_HINT)),
    ];
    if let Some(interaction) = fields.interaction.as_ref() {
        lines.insert(1, Line::from(format!("{INTERACTION_LABEL}: {}", interaction.prompt())));
        if fields.capabilities.respond_interaction == (Capability::Available {}) {
            for (index, choice) in interaction.choices().iter().take(9).enumerate() {
                lines.push(Line::from(format!("{}. {}", index + 1, choice.label)));
            }
            lines.push(Line::from(INTERACTION_ANSWER_HINT));
        }
    }
    lines.push(Line::from(DETAILS_CLOSE_HINT));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    true
}
