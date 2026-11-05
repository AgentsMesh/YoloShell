use crate::{constants::*, fixtures::Fixture, render_support::render};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::{Position, Rect},
};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_inventory::{HUB_UNREACHABLE_GLYPH, View};

#[test]
fn agent_without_session_metadata_and_explicitly_unreachable_hub_remain_visible() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    inventory.scope = Some(fixture.hub.hub_id);
    inventory.history = true;
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            render(
                frame,
                &inventory,
                &std::collections::BTreeMap::new(),
                None,
                "",
                frame.area(),
            )
        })
        .unwrap();
    let content = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(content.contains(PROVIDER_KIND));
    assert!(content.contains(&fixture.hub.hub_id.to_string()));
    inventory
        .machines
        .get_mut(&fixture.hub.hub_id)
        .unwrap()
        .reachable = false;
    inventory.view = View::Machines;
    terminal
        .draw(|frame| {
            render(
                frame,
                &inventory,
                &fixture.terminals(),
                None,
                "",
                frame.area(),
            )
        })
        .unwrap();
    let content = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(content.contains(HUB_UNREACHABLE_GLYPH));
    assert!(!content.contains("Unreachable"));
    assert!(content.contains("Current"));
}

#[test]
fn unicode_session_metadata_does_not_resize_or_escape_the_viewport() {
    let fixture = Fixture::new();
    let mut fields = fixture.session.fields().clone();
    fields.name = "\u{7ec8}\u{7aef} e\u{301}".into();
    fields.cwd = "/\u{5de5}\u{4f5c}\u{533a}".into();
    let terminals =
        std::collections::BTreeMap::from([(fields.address, TerminalSummary::new(fields).unwrap())]);
    let mut inventory = fixture.inventory();
    for width in [1, 2, 10, 20, WIDTH] {
        let mut terminal = Terminal::new(TestBackend::new(width, HEIGHT)).unwrap();
        for view in [View::Terminals, View::Agents] {
            inventory.view = view;
            terminal
                .draw(|frame| {
                    render(
                        frame,
                        &inventory,
                        &terminals,
                        Some(fixture.session.fields().address),
                        "",
                        frame.area(),
                    )
                })
                .unwrap();
            assert_eq!(terminal.backend().buffer().area.width, width);
            assert_eq!(terminal.backend().buffer().area.height, HEIGHT);
        }
    }
}

#[test]
fn compact_tabs_and_lists_never_write_outside_the_allocated_inventory_area() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    for (width, height) in [(0, 0), (1, 1), (2, 2), (10, 6), (20, 15), (80, 24)] {
        let area = Rect::new(2, 2, width, height);
        for view in [View::Terminals, View::Agents, View::Machines] {
            inventory.view = view;
            terminal
                .draw(|frame| {
                    for cell in &mut frame.buffer_mut().content {
                        cell.set_symbol(OUTSIDE_CELL);
                    }
                    render(
                        frame,
                        &inventory,
                        &terminals,
                        Some(fixture.session.fields().address),
                        "",
                        area,
                    );
                })
                .unwrap();
            for row in 0..HEIGHT {
                for column in 0..WIDTH {
                    if !area.contains(Position::new(column, row)) {
                        assert_eq!(
                            terminal.backend().buffer()[(column, row)].symbol(),
                            OUTSIDE_CELL
                        );
                    }
                }
            }
        }
    }
}
