use crate::{State, constants::*, terminal_status_constants::*};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use yolo_hub_agent_protocol::Activity;
use yolo_hub_terminal_protocol::TerminalSize;
use yolo_tui_lifecycle::{ConnectionState, InputAccess, TerminalPhase};

pub(crate) fn line(state: &State, desired: Option<TerminalSize>) -> Line<'static> {
    let focused = state.focused_terminal();
    let terminal = focused.and_then(|address| state.terminals.get(&address));
    let title = terminal.map_or(PRODUCT_NAME, |terminal| terminal.display_title());
    let mut spans = vec![Span::raw(title.to_owned())];
    let starting = crate::creation_presentation::starting_in_focused_pane(state);
    let Some(address) = focused else {
        if starting {
            append(
                &mut spans,
                crate::startup_constants::STARTING_SHELL,
                Color::Yellow,
            );
        } else if state.control_generation.is_none() {
            append(
                &mut spans,
                yolo_tui_lifecycle::HUB_RECONNECTING_LABEL,
                Color::Yellow,
            );
        }
        return Line::from(spans);
    };
    let projection = crate::terminal_lifecycle::projection(state, address, desired);
    let color = if projection.input == InputAccess::Writing {
        Color::Green
    } else if projection.is_warning() {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let label = if starting
        && matches!(
            projection.phase,
            TerminalPhase::Starting | TerminalPhase::Unavailable
        ) {
        crate::startup_constants::STARTING_SHELL.to_owned()
    } else if matches!(
        projection.input,
        InputAccess::ReadOnly | InputAccess::Writing
    ) {
        crate::control::detail(state, address)
    } else {
        projection.label().to_owned()
    };
    append(&mut spans, &label, color);
    if projection.phase != TerminalPhase::Running {
        if let Some(connection) = projection.connection.label() {
            append(&mut spans, connection, Color::Yellow);
        }
    }
    if projection.connection == ConnectionState::Connected
        && !matches!(
            projection.phase,
            TerminalPhase::Ended | TerminalPhase::Lost | TerminalPhase::Archived
        )
        && let Some(agent) = terminal.and_then(|terminal| {
            yolo_tui_lifecycle::active_agent(
                terminal.fields(),
                terminal
                    .fields()
                    .agent
                    .and_then(|target| state.inventory.agent_for_target(target)),
                state.inventory.current,
            )
        })
    {
        let activity = match agent.fields().activity {
            Activity::Working => WORKING_STATUS,
            Activity::WaitingForInput => WAITING_INPUT_STATUS,
            Activity::WaitingForApproval => WAITING_APPROVAL_STATUS,
            Activity::Idle | Activity::Unknown => EMPTY_STATUS,
        };
        spans.push(Span::raw(format!(
            "{STATUS_SEPARATOR}{}{activity}",
            agent
                .fields()
                .provider
                .display_name()
                .unwrap_or(agent.fields().provider.kind())
        )));
    }
    Line::from(spans)
}

fn append(spans: &mut Vec<Span<'static>>, label: &str, color: Color) {
    let separator = if label == yolo_tui_render::WRITING_LABEL {
        STATUS_SPACE
    } else {
        STATUS_SEPARATOR
    };
    spans.push(Span::styled(
        format!("{separator}{label}"),
        Style::default().fg(color),
    ));
}
