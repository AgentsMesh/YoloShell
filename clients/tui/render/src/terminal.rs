use crate::{TerminalWidget, constants::*};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
};
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;
use yolo_presentation_layout::{Axis, Layout, LayoutError};
use yolo_tui_lifecycle::{InputAccess, TerminalProjection};

pub struct TerminalPane<'a> {
    pub screen: Option<&'a vt100::Screen>,
    pub projection: TerminalProjection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaneViewport {
    pub pane: Uuid,
    pub terminal: Option<TerminalAddress>,
    pub inner: Rect,
    pub outer: Rect,
}

/// Reserves only internal divider cells; a single terminal loses no content cells.
pub fn terminal_inner(area: Rect, bounds: Rect) -> Rect {
    let left = if area.x > bounds.x {
        CELL_WIDTH.min(area.width)
    } else {
        0
    };
    let top = if area.y > bounds.y {
        CELL_WIDTH.min(area.height)
    } else {
        0
    };
    Rect::new(
        area.x + left,
        area.y + top,
        area.width - left,
        area.height - top,
    )
}

pub fn render_terminal(
    frame: &mut Frame<'_>,
    layout: &Layout,
    terminals: &BTreeMap<TerminalAddress, TerminalPane<'_>>,
    bounds: Rect,
    alternate: Option<uuid::Uuid>,
) -> Result<Vec<PaneViewport>, LayoutError> {
    let rect = yolo_presentation_layout::Rect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    };
    let geometry = layout.geometry(rect)?;
    let panes = layout.panes();
    let mut viewports = Vec::with_capacity(geometry.len());
    for placement in &geometry {
        let pane = panes
            .iter()
            .find(|pane| pane.id == placement.pane)
            .ok_or(LayoutError::UnknownPane)?;
        let area = Rect::new(
            placement.rect.x,
            placement.rect.y,
            placement.rect.width,
            placement.rect.height,
        );
        let inner = terminal_inner(area, bounds);
        let content = pane
            .terminal
            .filter(|_| alternate != Some(pane.id))
            .and_then(|address| terminals.get(&address));
        if let Some(screen) = content.and_then(|content| content.screen) {
            frame.render_widget(TerminalWidget(screen), inner);
            let (row, column) = screen.cursor_position();
            if pane.id == layout.focused()
                && content.is_some_and(|content| content.projection.input == InputAccess::Writing)
                && !screen.hide_cursor()
                && screen.scrollback() == 0
                && row < inner.height
                && column < inner.width
            {
                frame.set_cursor_position((inner.x + column, inner.y + row));
            }
        } else if pane.terminal.is_none() {
            frame.render_widget(
                Paragraph::new(EMPTY_PANE_LABEL).style(Style::default().fg(UNFOCUSED_BORDER)),
                inner,
            );
        }
        if let Some(content) = content {
            if let Some(hint) = content.projection.action_hint() {
                // This path has no pane border or title, so a retained screen
                // needs the explicit phase/action card to avoid looking live.
                crate::overlay::render_action_overlay(
                    frame,
                    inner,
                    content.projection.phase.label(),
                    hint,
                );
            }
        }
        viewports.push(PaneViewport {
            pane: pane.id,
            terminal: pane.terminal,
            inner,
            outer: area,
        });
    }
    let focused = viewports
        .iter()
        .find(|pane| pane.pane == layout.focused())
        .map(|pane| pane.outer);
    for split in layout.split_geometry(rect)? {
        let (area, border, adjacent) = match split.axis {
            Axis::Horizontal => (
                Rect::new(split.boundary, split.rect.y, CELL_WIDTH, split.rect.height),
                Borders::LEFT,
                focused
                    .is_some_and(|area| area.x == split.boundary || area.right() == split.boundary),
            ),
            Axis::Vertical => (
                Rect::new(split.rect.x, split.boundary, split.rect.width, CELL_WIDTH),
                Borders::TOP,
                focused.is_some_and(|area| {
                    area.y == split.boundary || area.bottom() == split.boundary
                }),
            ),
        };
        let area = area.intersection(bounds);
        frame.render_widget(
            Block::default()
                .borders(border)
                .border_style(Style::default().fg(UNFOCUSED_BORDER)),
            area,
        );
        if let Some(focused) = focused.filter(|_| adjacent) {
            let edge = match split.axis {
                Axis::Horizontal => Rect::new(area.x, focused.y, area.width, focused.height),
                Axis::Vertical => Rect::new(focused.x, area.y, focused.width, area.height),
            }
            .intersection(area);
            frame.render_widget(
                Block::default()
                    .borders(border)
                    .border_style(Style::default().fg(FOCUSED_BORDER)),
                edge,
            );
        }
    }
    Ok(viewports)
}
