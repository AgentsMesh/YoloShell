use crate::{
    constants::*,
    fields::Value,
    form::{Form, FormKind},
    rows::{self, Row, View},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
};
use uuid::Uuid;

pub(crate) fn render(frame: &mut Frame<'_>, form: &Form, area: Rect, epoch: Uuid) -> View {
    let title = match form.kind {
        FormKind::Peer(None) => ADD_PEER,
        FormKind::Peer(Some(_)) => EDIT_PEER,
        FormKind::Origin => ADD_ORIGIN,
        FormKind::Export { .. } => EXPORTS,
        FormKind::Listener => LISTENER,
        FormKind::FileRoot {
            existing: false, ..
        } => ADD_FILE_ROOT,
        FormKind::FileRoot { existing: true, .. } => EDIT_FILE_ROOT,
    };
    let areas = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(Color::Cyan)),
        areas[0],
    );
    let items = (0..form.rows())
        .map(|index| {
            let selected = index == form.focused;
            let prefix = if selected { SELECTED } else { UNSELECTED };
            let (text, read_only, cursor) = if let Some(field) = form.fields.get(index) {
                let label = format!("{prefix}{}: ", field.label());
                let cursor = match &field.value {
                    Value::Text { text, cursor } if selected && !field.read_only => {
                        Some(label.len() + cursor.position(text))
                    }
                    _ => None,
                };
                (
                    format!("{label}{} ", field.display()),
                    field.read_only,
                    cursor,
                )
            } else {
                let command = if index == form.fields.len() {
                    SAVE
                } else if index == form.fields.len() + STEP {
                    CANCEL
                } else {
                    form.removal_label()
                };
                (format!("{prefix}{command}"), false, None)
            };
            let style = if read_only {
                Style::default().fg(Color::DarkGray)
            } else if selected {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default()
            };
            Row {
                text,
                style,
                cursor,
            }
        })
        .collect();
    rows::render(frame, areas[1], items, form.focused, epoch)
}
