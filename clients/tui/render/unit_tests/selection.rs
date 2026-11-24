use crate::constants::*;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use yolo_tui_render::{PaneSelection, PaneViewport, highlight_selection};

#[test]
fn selection_overlay_clips_to_terminal_cells_and_never_highlights_pane_borders() {
    let pane = uuid::Uuid::new_v4();
    let view = PaneViewport {
        pane,
        terminal: None,
        outer: Rect::new(u16::default(), u16::default(), COLUMNS, ROWS),
        inner: Rect::new(INNER_INSET, INNER_INSET, SHORT_EXTENT, SHORT_EXTENT),
    };
    let selected = PaneSelection {
        pane,
        start: START_CELL,
        end: END_CELL,
    };
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    terminal
        .draw(|frame| highlight_selection(frame, view, selected))
        .unwrap();
    let buffer = terminal.backend().buffer();
    for row in u16::default()..ROWS {
        for column in u16::default()..COLUMNS {
            let coordinate = (
                row.saturating_sub(INNER_INSET),
                column.saturating_sub(INNER_INSET),
            );
            let expected = view.inner.contains((column, row).into())
                && coordinate >= START_CELL
                && coordinate <= END_CELL;
            assert_eq!(buffer[(column, row)].bg == SELECTED_BACKGROUND, expected);
        }
    }
}

#[test]
fn invalid_or_foreign_selection_cannot_color_unrelated_content() {
    let pane = uuid::Uuid::new_v4();
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    for selected in [
        PaneSelection {
            pane: uuid::Uuid::new_v4(),
            start: START_CELL,
            end: END_CELL,
        },
        PaneSelection {
            pane,
            start: END_CELL,
            end: START_CELL,
        },
        PaneSelection {
            pane,
            start: (u16::MAX, u16::MAX),
            end: (u16::MAX, u16::MAX),
        },
    ] {
        terminal
            .draw(|frame| {
                let view = PaneViewport {
                    pane,
                    terminal: None,
                    inner: frame.area(),
                    outer: frame.area(),
                };
                highlight_selection(frame, view, selected);
            })
            .unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .all(|cell| cell.bg != SELECTED_BACKGROUND)
        );
    }
    terminal
        .draw(|frame| {
            let view = PaneViewport {
                pane,
                terminal: None,
                inner: Rect::default(),
                outer: frame.area(),
            };
            highlight_selection(
                frame,
                view,
                PaneSelection {
                    pane,
                    start: START_CELL,
                    end: END_CELL,
                },
            );
        })
        .unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .all(|cell| cell.bg != SELECTED_BACKGROUND)
    );
}
