use crate::{Form, Manager, constants::*};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{List, ListState, Paragraph},
};
use yolo_tui_tab::TabCatalog;

pub fn render(frame: &mut Frame<'_>, manager: &Manager, tabs: &TabCatalog, area: Rect) {
    let areas = Layout::vertical([
        Constraint::Length(HEADER_ROWS),
        Constraint::Min(0),
        Constraint::Length(FOOTER_ROWS),
    ])
    .split(area);
    frame.render_widget(Paragraph::new(format!("{TITLE}\n{HELP}")), areas[0]);
    let rows = tabs
        .machine_entries()
        .iter()
        .map(|entry| {
            format!(
                "{}{}",
                if entry.id == tabs.active().id {
                    ACTIVE_MARK
                } else {
                    INACTIVE_MARK
                },
                entry.name
            )
        })
        .collect::<Vec<_>>();
    let mut selection = ListState::default().with_selected(
        tabs.machine_entries()
            .iter()
            .position(|entry| entry.id == manager.selected),
    );
    let mut list = List::new(rows).highlight_style(Style::default().fg(Color::Cyan));
    if ratatui::text::Line::from(SELECTED_MARK).width() <= usize::from(areas[1].width) {
        list = list.highlight_symbol(SELECTED_MARK);
    }
    frame.render_stateful_widget(list, areas[1], &mut selection);
    if matches!(manager.form, Some(Form::Create | Form::Rename(_))) {
        let label = if manager.form == Some(Form::Create) {
            CREATE_LABEL
        } else {
            RENAME_LABEL
        };
        let field = Rect::new(
            areas[2].x,
            areas[2].y,
            areas[2].width,
            areas[2].height.min(1),
        );
        yolo_tui_line_input::render(frame, field, label, "", &manager.text, &manager.cursor);
        if areas[2].height > field.height {
            frame.render_widget(
                Paragraph::new(manager.message.as_str()),
                Rect::new(
                    areas[2].x,
                    areas[2].y + field.height,
                    areas[2].width,
                    areas[2].height - field.height,
                ),
            );
        }
        return;
    }
    let prompt = match manager.form {
        Some(Form::Remove(_)) => REMOVE_LABEL.into(),
        _ => String::new(),
    };
    frame.render_widget(
        Paragraph::new(format!("{prompt}\n{}", manager.message)),
        areas[2],
    );
}
