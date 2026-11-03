use crate::{Mode, State, constants::*};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_terminal_protocol::TerminalSize;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    state: &State,
    area: Rect,
    desired: Option<TerminalSize>,
) {
    if state.prefix {
        let hint = if state.client_capability_enabled(ClientCapability::TerminalLifecycle) {
            PREFIX_HINT
        } else {
            OBSERVER_PREFIX_HINT
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::Cyan)),
            area,
        );
        return;
    }
    if crate::form_input::draw(frame, state, area) {
        return;
    }
    if matches!(state.mode, Mode::Terminal | Mode::Chat) {
        let mut line = crate::terminal_status::line(state, desired);
        if let Some(message) = important(state) {
            if message != crate::startup_constants::STARTING_SHELL {
                line.spans.push(Span::styled(
                    format!(" | {message}"),
                    Style::default().fg(Color::Yellow),
                ));
            }
        }
        yolo_tui_render::draw_status_line(
            frame,
            line,
            area,
            crate::terminal_input_authority::status(state).is_some(),
        );
        return;
    }
    let mut status = match state.mode {
        Mode::ConfirmTerminate => {
            confirmation(state, "End", " and its programs?", "terminate", area.width)
        }
        Mode::ConfirmArchive => confirmation(state, "Archive", "?", "archive", area.width),
        Mode::ConfirmRecreate => recreate_confirmation(state, area.width),
        Mode::Document => yolo_tui_document::HINT.into(),
        _ => match important(state) {
            Some(message) => message,
            None if !state.client_capability_enabled(ClientCapability::TerminalLifecycle) => {
                OBSERVER_INVENTORY_HINT.into()
            }
            None if state.terminals.is_empty() => EMPTY_INVENTORY_HINT.into(),
            None => INVENTORY_HINT.into(),
        },
    };
    if state.mode == Mode::Inventory {
        status = inventory_status(state, status);
    }
    let title = (state.mode == Mode::Inventory && area.width >= MIN_INVENTORY_TITLE_WIDTH)
        .then(|| selected_title(state))
        .flatten();
    let mut status = status;
    append_retained(state, &mut status);
    if let Some(title) = title {
        yolo_tui_render::draw_status_line(
            frame,
            Line::from(vec![
                Span::raw(title.to_owned()),
                Span::raw(format!(" | {status}")),
            ]),
            area,
            false,
        );
    } else {
        frame.render_widget(Paragraph::new(status), area);
    }
}

fn inventory_status(state: &State, status: String) -> String {
    if status == yolo_tui_lifecycle::ENDED_HINT {
        return ENDED_INVENTORY_HINT.into();
    }
    if status == yolo_tui_lifecycle::UNAVAILABLE_HINT {
        return UNAVAILABLE_INVENTORY_HINT.into();
    }
    if status == HISTORY_SCOPE_LABEL || status == INVENTORY_HINT {
        return history_hint(state).to_owned();
    }
    if status == EMPTY_INVENTORY_HINT {
        return if state.inventory.history {
            history_hint(state)
        } else {
            EMPTY_INVENTORY_HINT
        }
        .to_owned();
    }
    if status == ACTIVE_SCOPE_LABEL {
        return INVENTORY_HINT.into();
    }
    status
}

fn history_hint(state: &State) -> &'static str {
    if !state.client_capability_enabled(ClientCapability::TerminalLifecycle) {
        return OBSERVER_INVENTORY_HINT;
    }
    if state.inventory.history {
        let archived = state.selected.is_some_and(|address| {
            state.terminals.get(&address).is_some_and(|terminal| {
                matches!(
                    state.inventory.phase(address, terminal.fields()),
                    yolo_tui_lifecycle::TerminalPhase::Archived
                )
            })
        });
        return if archived {
            ARCHIVED_HISTORY_HINT
        } else {
            HISTORY_INVENTORY_HINT
        };
    }
    INVENTORY_HINT
}

fn confirmation(state: &State, verb: &str, suffix: &str, key: &str, width: u16) -> String {
    let target = confirmation_target(state).unwrap_or("selected terminal");
    let prefix = format!("{verb} ");
    let suffix = format!("{suffix} Enter {key} · Esc cancel");
    format!(
        "{prefix}{}{suffix}",
        elide_target(target, &prefix, &suffix, width)
    )
}

fn recreate_confirmation(state: &State, width: u16) -> String {
    let Some(target) = confirmation_target(state) else {
        return crate::terminal_recovery_actions::CONFIRM_RECREATE.into();
    };
    let prefix = "New shell for ";
    let suffix = "; agent will not resume. Enter confirms · Esc cancels";
    format!(
        "{prefix}{}{suffix}",
        elide_target(target, prefix, suffix, width)
    )
}

fn elide_target<'a>(
    target: &'a str,
    prefix: &str,
    suffix: &str,
    width: u16,
) -> std::borrow::Cow<'a, str> {
    let fixed = Line::raw(prefix).width() + Line::raw(suffix).width();
    let available = usize::from(width).saturating_sub(fixed);
    if available < crate::constants::MIN_CONFIRM_TARGET_COLUMNS {
        return std::borrow::Cow::Borrowed("");
    }
    std::borrow::Cow::Owned(yolo_tui_render::elide(target, available))
}

fn confirmation_target(state: &State) -> Option<&str> {
    state
        .selected
        .and_then(|address| state.terminals.get(&address))
        .map(|terminal| terminal.display_title())
        .or_else(|| selected_title(state))
}

pub(crate) fn append_retained(state: &State, status: &mut String) {
    let blocked = crate::finalization_status::blocked(state);
    if blocked > 0 && !status.contains(crate::finalization_status::BLOCKED_LABEL) {
        status.push_str(&format!(
            " | {}: {blocked}",
            crate::finalization_status::BLOCKED_LABEL
        ));
    }
    if state.creations.persistence_error.is_some() {
        status.push_str(&format!(" | {UNSAVED_SHELL_CHANGES_LABEL}"));
    }
}

include!("status_bar_labels.rs");
