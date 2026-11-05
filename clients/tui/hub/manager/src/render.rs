use crate::{
    Manager,
    constants::*,
    model::Page,
    rows::{self, Row, View},
    wire_fields::key_text,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Paragraph, Wrap},
};

pub fn render(frame: &mut Frame<'_>, manager: &Manager, area: Rect) -> View {
    let identity = manager
        .base
        .as_ref()
        .map(|base| {
            let identity = format!(
                "{TITLE}  revision {}\n{HUB_ID}: {}\n{PUBLIC_KEY}: {}",
                base.state.revision,
                base.state.identity.hub(),
                key_text(base.state.identity)
            );
            match &base.peer_socket {
                Some(path) => format!("{identity}\n{LISTENING}: {}", path.display()),
                None => identity,
            }
        })
        .unwrap_or_else(|| format!("{TITLE}\n{HUB_ID}: {}", manager.entry));
    let header = rows::wrap(&identity, area.width);
    let header_height = u16::try_from(header.len())
        .unwrap_or(u16::MAX)
        .min(area.height.saturating_sub(STATUS_ROWS + MIN_CONTENT_ROWS));
    let bands = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Min(0),
        Constraint::Length(STATUS_ROWS),
    ])
    .split(area);
    frame.render_widget(Paragraph::new(header.join("\n")), bands[0]);
    let view = if let Some(confirmation) = &manager.confirmation {
        let label = rows::wrap(confirmation.label, bands[1].width);
        let height = u16::try_from(label.len())
            .unwrap_or(u16::MAX)
            .min(bands[1].height);
        let areas =
            Layout::vertical([Constraint::Length(height), Constraint::Min(0)]).split(bands[1]);
        frame.render_widget(Paragraph::new(label.join("\n")), areas[0]);
        let selected = usize::from(confirmation.confirm);
        let items = [CANCEL, CONFIRM]
            .into_iter()
            .enumerate()
            .map(|(index, text)| Row {
                text: format!(
                    "{}{}",
                    if index == selected {
                        SELECTED
                    } else {
                        UNSELECTED
                    },
                    text
                ),
                style: Style::default(),
                cursor: None,
            })
            .collect();
        rows::render(frame, areas[1], items, selected, manager.epoch)
    } else if let Some(form) = &manager.form {
        crate::form_render::render(frame, form, bands[1], manager.epoch)
    } else {
        menu(frame, manager, bands[1])
    };
    frame.render_widget(
        Paragraph::new(manager.status.as_str())
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Yellow)),
        bands[2],
    );
    view
}
fn menu(frame: &mut Frame<'_>, manager: &Manager, area: Rect) -> View {
    let page = match manager.page {
        Page::Root => TITLE.into(),
        Page::Peers => CONNECTIONS.into(),
        Page::Origins => ORIGINS.into(),
        Page::FileRoots => FILE_ROOTS.into(),
        Page::Peer(hub) => format!("{CONNECTIONS}: {hub}"),
        Page::Exports(hub) => format!("{EXPORTS}: {hub}"),
    };
    let heading = rows::wrap(&page, area.width);
    let height = u16::try_from(heading.len())
        .unwrap_or(u16::MAX)
        .min(area.height);
    let areas = Layout::vertical([Constraint::Length(height), Constraint::Min(0)]).split(area);
    frame.render_widget(
        Paragraph::new(heading.join("\n")).style(Style::default().fg(Color::Cyan)),
        areas[0],
    );
    let items = manager
        .items()
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let selected = index == manager.selected;
            let style = if !item.enabled {
                Style::default().fg(Color::DarkGray)
            } else if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row {
                text: format!(
                    "{}{}",
                    if selected { SELECTED } else { UNSELECTED },
                    item.label
                ),
                style,
                cursor: None,
            }
        })
        .collect();
    rows::render(frame, areas[1], items, manager.selected, manager.epoch)
}
