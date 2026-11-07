use crate::{constants::*, fixtures::Fixture, render_support::render};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::{Position, Rect},
};
use yolo_hub_agent_protocol::{AgentSnapshot, AgentWorkingDirectory, Freshness, Reachability};
use yolo_tui_inventory::{Inventory, View, render_details};

fn rendered(fixture: &Fixture, inventory: &Inventory) -> String {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            render(
                frame,
                inventory,
                &fixture.terminals(),
                None,
                "",
                frame.area(),
            )
        })
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn rendered_details(fixture: &Fixture, inventory: &Inventory) -> String {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            assert!(render_details(
                frame,
                inventory,
                &fixture.terminals(),
                None,
                frame.area()
            ));
        })
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn current_agent_row_stays_one_line_and_details_show_its_directory() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    let row = rendered(&fixture, &inventory);
    assert!(!row.contains(AGENT_WORKING_DIRECTORY));
    assert!(!row.contains(DIRECTORY_LABEL));
    assert!(!row.contains(WORKING_DIRECTORY));

    inventory.details = Some(fixture.agent.cursor().target());
    let details = rendered_details(&fixture, &inventory);
    assert!(details.contains(AGENT_WORKING_DIRECTORY));
    assert!(details.contains(DIRECTORY_LABEL));
    assert!(!details.contains(WORKING_DIRECTORY));
    assert!(!details.contains(LAST_KNOWN_DIRECTORY_LABEL));
}

#[test]
fn unavailable_or_stale_directory_is_retained_with_explicit_last_known_marker() {
    let fixture = Fixture::new();
    for (current, freshness, reachability) in [
        (false, Freshness::Current, Reachability::Reachable),
        (true, Freshness::Stale, Reachability::Reachable),
        (true, Freshness::Current, Reachability::Unreachable),
        (true, Freshness::Stale, Reachability::Unreachable),
    ] {
        let mut fields = fixture.agent.fields().clone();
        fields.freshness = freshness;
        fields.reachability = reachability;
        let mut inventory = fixture.inventory();
        inventory.current = current;
        inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
        inventory.history = true;
        inventory.details = Some(fixture.agent.cursor().target());
        let row = rendered(&fixture, &inventory);
        assert!(!row.contains(AGENT_WORKING_DIRECTORY));
        let details = rendered_details(&fixture, &inventory);
        assert!(details.contains(AGENT_WORKING_DIRECTORY));
        assert!(details.contains(LAST_KNOWN_DIRECTORY_LABEL));
    }
}

#[test]
fn unknown_directory_has_an_explicit_label_even_with_an_associated_shell() {
    let fixture = Fixture::new();
    let mut fields = fixture.agent.fields().clone();
    fields.working_directory = None;
    let mut inventory = fixture.inventory();
    inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
    inventory.details = Some(fixture.agent.cursor().target());
    let row = rendered(&fixture, &inventory);
    assert!(!row.contains(UNKNOWN_DIRECTORY_LABEL));
    let details = rendered_details(&fixture, &inventory);
    assert!(details.contains(UNKNOWN_DIRECTORY_LABEL));
    assert!(!details.contains(WORKING_DIRECTORY));
    assert!(!details.contains(AGENT_WORKING_DIRECTORY));
}

#[test]
fn literal_directory_does_not_execute_escapes_or_write_outside_inventory() {
    let fixture = Fixture::new();
    let mut fields = fixture.agent.fields().clone();
    fields.working_directory =
        Some(AgentWorkingDirectory::new(LITERAL_AGENT_DIRECTORY.into()).unwrap());
    let mut inventory = fixture.inventory();
    inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
    inventory.details = Some(fixture.agent.cursor().target());
    let text = rendered_details(&fixture, &inventory);
    for part in LITERAL_AGENT_DIRECTORY.split_whitespace() {
        if part.is_ascii() {
            assert!(text.contains(part));
        } else {
            for character in part.chars() {
                assert!(text.contains(character));
            }
        }
    }
    for width in [1, 2, 10, WIDTH / 2] {
        let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
        let area = Rect::new(2, 2, width, HEIGHT / 2);
        terminal
            .draw(|frame| {
                for cell in &mut frame.buffer_mut().content {
                    cell.set_symbol(OUTSIDE_CELL);
                }
                render(frame, &inventory, &fixture.terminals(), None, "", area);
            })
            .unwrap();
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                if !area.contains(Position::new(col, row)) {
                    assert_eq!(
                        terminal.backend().buffer()[(col, row)].symbol(),
                        OUTSIDE_CELL,
                        "inventory width={width}, outside column={col}, row={row}"
                    );
                }
            }
        }
    }
}

#[test]
fn narrow_selected_rows_in_every_inventory_view_never_overwrite_adjacent_content() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    for view in [View::Terminals, View::Agents, View::Hubs] {
        inventory.view = view;
        for width in [0, 1, 2] {
            let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
            let area = Rect::new(2, 2, width, HEIGHT / 2);
            terminal
                .draw(|frame| {
                    for cell in &mut frame.buffer_mut().content {
                        cell.set_symbol(OUTSIDE_CELL);
                    }
                    render(
                        frame,
                        &inventory,
                        &fixture.terminals(),
                        Some(fixture.session.fields().address),
                        "",
                        area,
                    );
                })
                .unwrap();
            for row in 0..HEIGHT {
                for col in 0..WIDTH {
                    if !area.contains(Position::new(col, row)) {
                        assert_eq!(
                            terminal.backend().buffer()[(col, row)].symbol(),
                            OUTSIDE_CELL,
                            "view={view:?}, width={width}, outside column={col}, row={row}"
                        );
                    }
                }
            }
        }
    }
}
