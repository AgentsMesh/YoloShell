use crate::{
    constants::*,
    picker::{Kind, Picker},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use yolo_hub_workspace_protocol::{WorkspaceKind, WorkspaceSummary};

pub fn render(frame: &mut Frame<'_>, picker: &Picker) {
    let area = centered(frame.area());
    let block = Block::default().borders(Borders::ALL).title(TITLE);
    let body = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(body);
    let mut items = vec![ListItem::new(all_text(picker.current().is_none()))];
    items.extend(
        picker
            .workspaces()
            .iter()
            .map(|workspace| ListItem::new(workspace_text(workspace, picker.current()))),
    );
    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(SELECTED_PREFIX);
    let mut state = ListState::default().with_selected(Some(picker.selected()));
    frame.render_stateful_widget(list, rows[0], &mut state);
    if picker.is_pending() {
        frame.render_widget(Paragraph::new(SAVING), rows[1]);
    } else if let Some(draft) = picker.draft() {
        let (label, hint) = match draft.kind() {
            Kind::Create => (
                NEW_WORKSPACE_LABEL,
                picker
                    .message()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("{ROOT_HINT}{}", draft.root().unwrap_or_default())),
            ),
            Kind::Rename => (
                RENAME_LABEL,
                picker.message().unwrap_or(RENAME_HINT).to_owned(),
            ),
            Kind::Labels => (
                LABELS_LABEL,
                picker.message().unwrap_or(LABELS_HINT).to_owned(),
            ),
        };
        yolo_tui_line_input::render(frame, rows[1], label, &hint, &draft.text, &draft.cursor);
    } else {
        frame.render_widget(Paragraph::new(footer(picker)), rows[1]);
    }
}

fn all_text(current: bool) -> String {
    if current {
        format!("{ALL_WORKSPACES}{CURRENT_TAG}")
    } else {
        ALL_WORKSPACES.into()
    }
}

fn workspace_text(workspace: &WorkspaceSummary, current: Option<yolo_hub_identity::WorkspaceAddress>) -> String {
    let kind = match workspace.kind {
        WorkspaceKind::Directory => DIRECTORY_KIND,
        WorkspaceKind::Worktree => WORKTREE_KIND,
    };
    let current = if current == Some(workspace.address) {
        CURRENT_TAG
    } else {
        ""
    };
    format!(
        "{}{SEPARATOR}{kind}{SEPARATOR}{}{TERMINALS_SUFFIX}{SEPARATOR}{}{RUNS_SUFFIX}{SEPARATOR}{}{current}",
        workspace.name, workspace.member_terminals, workspace.member_runs, workspace.root
    )
}

fn footer(picker: &Picker) -> String {
    if let Some(message) = picker.message() {
        message.to_owned()
    } else if picker.is_loading() {
        LOADING.into()
    } else {
        FOOTER.into()
    }
}

fn centered(area: Rect) -> Rect {
    let width = area.width.saturating_mul(PALETTE_NUMERATOR) / PALETTE_DENOMINATOR;
    let height = area.height.saturating_mul(PALETTE_NUMERATOR) / PALETTE_DENOMINATOR;
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}
