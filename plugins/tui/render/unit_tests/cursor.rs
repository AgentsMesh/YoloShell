use crate::constants::*;
use ratatui::{Terminal, backend::TestBackend};
use std::collections::BTreeMap;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_presentation_layout::{Axis, Layout};
use yolo_tui_render::{TerminalPane, render_terminal};

#[test]
fn writing_cursor_stays_hidden_in_scrollback_or_outside_the_current_pane_view() {
    for condition in [
        Condition::Live,
        Condition::History,
        Condition::Below,
        Condition::Right,
    ] {
        let address = TerminalAddress {
            hub_id: HubId::new(),
            terminal_id: TerminalId::new(),
        };
        let mut layout = Layout::new(Some(address));
        match condition {
            Condition::Below => {
                layout.split(Axis::Vertical, Some(address)).unwrap();
            }
            Condition::Right => {
                layout.split(Axis::Horizontal, Some(address)).unwrap();
            }
            Condition::Live | Condition::History => {}
        }
        let mut parser = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK_ROWS);
        match condition {
            Condition::Live => parser.process(SCREEN_TEXT),
            Condition::History => {
                parser.process(&SCROLLED_LINE.repeat(usize::from(ROWS) + HISTORY_STEP));
                parser.screen_mut().set_scrollback(HISTORY_STEP);
                assert!(parser.screen().scrollback() > 0);
            }
            Condition::Below => parser.process(CURSOR_BELOW_VIEW),
            Condition::Right => parser.process(CURSOR_RIGHT_OF_VIEW),
        }
        let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
        terminal
            .draw(|frame| {
                let terminals = BTreeMap::from([(
                    address,
                    TerminalPane {
                        screen: Some(parser.screen()),
                        projection: WRITING,
                    },
                )]);
                render_terminal(frame, &layout, &terminals, frame.area(), None).unwrap();
            })
            .unwrap();
        assert_eq!(
            terminal.backend().cursor_visible(),
            matches!(condition, Condition::Live)
        );
    }
}

#[derive(Clone, Copy)]
enum Condition {
    Live,
    History,
    Below,
    Right,
}
