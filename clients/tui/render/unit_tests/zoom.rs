use crate::constants::*;
use ratatui::{Terminal, backend::TestBackend};
use std::collections::BTreeMap;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_presentation_layout::{Axis, Layout};
use yolo_tui_render::{TerminalPane, render_terminal};

#[test]
fn zoom_renders_one_full_viewport_with_the_original_pane_and_session() {
    let first = TerminalAddress {
        hub_id: HubId::new(),
        terminal_id: TerminalId::new(),
    };
    let second = TerminalAddress {
        hub_id: first.hub_id,
        terminal_id: TerminalId::new(),
    };
    let mut owner = Layout::new(Some(first));
    owner.split(Axis::Horizontal, Some(second)).unwrap();
    let before = owner.clone();
    let panes = BTreeMap::from([
        (
            first,
            TerminalPane {
                screen: None,
                projection: WRITING,
            },
        ),
        (
            second,
            TerminalPane {
                screen: None,
                projection: WRITING,
            },
        ),
    ]);
    let mut terminal = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    terminal
        .draw(|frame| {
            let view = owner.focused_view();
            let areas = render_terminal(frame, &view, &panes, frame.area(), None).unwrap();
            assert_eq!(areas.len(), SINGLE_VIEWPORT);
            assert_eq!(areas[usize::default()].pane, owner.focused());
            assert_eq!(areas[usize::default()].terminal, Some(second));
            assert_eq!(areas[usize::default()].outer, frame.area());
        })
        .unwrap();
    assert_eq!(owner, before);
    terminal
        .draw(|frame| {
            assert_eq!(
                render_terminal(frame, &owner, &panes, frame.area(), None)
                    .unwrap()
                    .len(),
                owner.panes().len()
            );
        })
        .unwrap();
}
