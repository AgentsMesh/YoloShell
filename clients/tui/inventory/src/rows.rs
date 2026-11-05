use crate::{
    constants::*,
    labels::{self, ControlBadge},
};
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::ListItem,
};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{Activity, AgentSnapshot, Freshness, Lifecycle, Reachability};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_rpc_protocol::HubSummary;
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_lifecycle::TerminalPhase;

pub(crate) fn terminal(
    session: &TerminalSummary,
    phase: TerminalPhase,
    attention: bool,
    width: u16,
) -> ListItem<'static> {
    let fields = session.fields();
    let row_width = usize::from(width).saturating_sub(Line::raw(SELECTION_MARKER).width());
    let marker = marker(attention);
    let label = phase.label();
    let fixed =
        Line::raw(marker).width() + Line::raw(PHASE_SEPARATOR).width() + Line::raw(label).width();
    let title = elide(session.display_title(), row_width.saturating_sub(fixed));
    let title = if title.is_empty() {
        format!("{marker}{label}")
    } else {
        format!("{marker}{title}{PHASE_SEPARATOR}{label}")
    };
    ListItem::new(vec![
        Line::from(title),
        muted(elide(&fields.cwd, row_width)),
        muted(format!(
            "{MACHINE_LABEL} {} · {TERMINAL_LABEL} {}",
            compact_id(&fields.address.hub_id.to_string()),
            compact_id(&fields.address.terminal_id.to_string())
        )),
    ])
}

/// DESIGN DIRECTION: terminal-native density.
/// One-line rows, fixed-width status slots, monochrome glyphs and no decoration.
pub(crate) fn agent(
    snapshot: &AgentSnapshot,
    terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    current: bool,
    attention: bool,
    control: Option<ControlBadge>,
    animation_phase: u8,
    width: u16,
) -> ListItem<'static> {
    let fields = snapshot.fields();
    let row_width = usize::from(width).saturating_sub(Line::raw(SELECTION_MARKER).width());
    let marker = marker(attention);
    let warning = !current
        || fields.freshness == Freshness::Stale
        || fields.reachability == Reachability::Unreachable;
    let live = matches!(
        fields.lifecycle,
        Lifecycle::Starting {} | Lifecycle::Running {}
    );
    let badge = live.then_some(control.unwrap_or(ControlBadge::Unavailable));
    let activity = match fields.lifecycle {
        Lifecycle::Starting {} => Some(labels::spinner_frame(animation_phase)),
        Lifecycle::Running {} if fields.activity == Activity::Working => {
            Some(labels::spinner_frame(animation_phase))
        }
        Lifecycle::Running {} if fields.activity == Activity::WaitingForInput => {
            Some(CONTROL_PAUSED_GLYPH)
        }
        Lifecycle::Running {} if fields.activity == Activity::WaitingForApproval => {
            Some(HEALTH_WARNING_GLYPH)
        }
        _ => None,
    };
    let warning_text = warning.then(|| format!("  {HEALTH_WARNING_GLYPH}"));
    let badge_text = badge.map(|badge| format!("  {}", badge.glyph()));
    let activity_text = activity.map(|activity| format!(" {activity}"));
    let fixed = Line::raw(marker).width()
        + warning_text
            .as_deref()
            .map_or(0, |text| Line::raw(text).width())
        + badge_text
            .as_deref()
            .map_or(0, |text| Line::raw(text).width())
        + activity_text
            .as_deref()
            .map_or(0, |text| Line::raw(text).width());
    let title = elide(
        crate::agent_title(snapshot, terminals),
        row_width.saturating_sub(fixed),
    );
    let mut spans = vec![Span::raw(marker), Span::raw(title)];
    if let Some(warning_text) = warning_text {
        spans.push(Span::styled(
            warning_text,
            Style::default().fg(WARNING_COLOR),
        ));
    }
    if let (Some(badge), Some(badge_text)) = (badge, badge_text) {
        let style = match badge {
            ControlBadge::Writable => Style::default(),
            ControlBadge::Paused => Style::default().fg(WARNING_COLOR),
            ControlBadge::ReadOnly | ControlBadge::Syncing | ControlBadge::Unavailable => {
                Style::default().fg(MUTED_COLOR)
            }
        };
        spans.push(Span::styled(badge_text, style));
    }
    if let (Some(activity), Some(activity_text)) = (activity, activity_text) {
        let waiting = matches!(
            fields.activity,
            Activity::WaitingForInput | Activity::WaitingForApproval
        );
        let style = if waiting {
            Style::default().fg(WARNING_COLOR)
        } else {
            Style::default()
        };
        spans.push(Span::styled(activity_text, style));
    }
    ListItem::new(Line::from(spans))
}

pub(crate) fn machine(
    hub: &HubSummary,
    current: bool,
    local_name: Option<&str>,
    width: u16,
) -> ListItem<'static> {
    let row_width = usize::from(width).saturating_sub(Line::raw(SELECTION_MARKER).width());
    let (glyph, style) = if !current {
        (HUB_UNKNOWN_GLYPH, Style::default().fg(MUTED_COLOR))
    } else if hub.reachable {
        (HUB_REACHABLE_GLYPH, Style::default())
    } else {
        (HUB_UNREACHABLE_GLYPH, Style::default().fg(WARNING_COLOR))
    };
    let name = local_name
        .or(hub.label.as_deref())
        .map_or_else(|| compact_id(&hub.hub_id.to_string()).to_owned(), str::to_owned);
    let prefix = if local_name.is_some() {
        format!("{THIS_MACHINE_LABEL} ")
    } else {
        format!("{MACHINE_LABEL} ")
    };
    let suffix = format!(" {glyph}");
    let fixed = Line::raw(&prefix).width() + Line::raw(&suffix).width();
    let name = elide(&name, row_width.saturating_sub(fixed));
    ListItem::new(Line::from(vec![
        Span::raw(format!("{prefix}{name}")),
        Span::styled(suffix, style),
    ]))
}

fn muted(value: String) -> Line<'static> {
    Line::from(value).style(Style::default().fg(MUTED_COLOR))
}

fn marker(attention: bool) -> &'static str {
    if attention { NOTIFICATION_MARKER } else { "" }
}
fn compact_id(value: &str) -> &str {
    value.get(..8).unwrap_or(value)
}

fn elide(value: &str, columns: usize) -> String {
    if columns == 0 {
        return String::new();
    }
    if Line::raw(value).width() <= columns {
        return value.to_owned();
    }
    let mut result = String::new();
    let mut used = 0usize;
    for character in value.chars() {
        let width = Line::raw(character.to_string()).width();
        if used.saturating_add(width).saturating_add(1) > columns {
            break;
        }
        result.push(character);
        used = used.saturating_add(width);
    }
    result.push('…');
    result
}
