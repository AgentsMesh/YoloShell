use super::{constants::*, fixtures::Fixture};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use std::collections::BTreeMap;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_inventory::{Inventory, InventoryViewport, View, render_viewport};

pub fn catalogue() -> (Inventory, BTreeMap<TerminalAddress, TerminalSummary>) {
    let fixtures = (0..POINTER_ITEMS)
        .map(|_| Fixture::new())
        .collect::<Vec<_>>();
    let terminals = fixtures
        .iter()
        .map(|fixture| (fixture.session.fields().address, fixture.session.clone()))
        .collect();
    let mut inventory = Inventory::default();
    inventory.replace(
        Some(
            fixtures
                .iter()
                .map(|fixture| fixture.agent.clone())
                .collect(),
        ),
        Some(fixtures.iter().map(|fixture| fixture.hub.clone()).collect()),
    );
    inventory.current = true;
    inventory.view = View::Terminals;
    (inventory, terminals)
}

pub fn render(
    inventory: &Inventory,
    terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    selected: Option<TerminalAddress>,
    filter: &str,
    area: Rect,
) -> InventoryViewport {
    let mut terminal = Terminal::new(TestBackend::new(POINTER_WIDTH, POINTER_HEIGHT)).unwrap();
    let mut viewport = None;
    terminal
        .draw(|frame| {
            viewport = Some(render_viewport(
                frame,
                inventory,
                terminals,
                selected,
                filter,
                &BTreeMap::new(),
                0,
                inventory.local_hub,
                area,
            ))
        })
        .unwrap();
    viewport.unwrap()
}

pub fn area() -> Rect {
    Rect::new(
        POINTER_INSET,
        POINTER_INSET,
        POINTER_WIDTH - POINTER_INSET,
        POINTER_HEIGHT - POINTER_INSET,
    )
}
