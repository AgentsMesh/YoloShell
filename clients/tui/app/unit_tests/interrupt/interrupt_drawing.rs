use super::interrupt_support::*;
use crate::{constants::QUEUE_CAPACITY, interrupt_constants as text};
use crossterm::event::KeyEventKind;
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::CommandOutcome;

#[test]
fn agent_inventory_renders_hotkey_and_correlated_delivery_or_unknown_status() {
    let mut state = ready();
    let (jobs, mut incoming) = mpsc::channel(QUEUE_CAPACITY);
    let mut screen = ratatui::Terminal::new(ratatui::backend::TestBackend::new(
        SCREEN_COLUMNS,
        SCREEN_ROWS,
    ))
    .unwrap();
    screen
        .draw(|frame| {
            crate::draw::draw(frame, &state);
        })
        .unwrap();
    assert!(contents(&screen).contains(text::HINT));
    press(&mut state, &jobs, KeyEventKind::Press);
    let pending = queued(&mut incoming);
    for (outcome, expected) in [
        (CommandOutcome::Unknown {}, text::UNKNOWN),
        (CommandOutcome::Delivered {}, text::SENT),
    ] {
        finish(&mut state, &jobs, pending.clone(), reply(&pending, outcome));
        screen
            .draw(|frame| {
                crate::draw::draw(frame, &state);
            })
            .unwrap();
        assert!(contents(&screen).contains(expected));
    }
}
fn contents(screen: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
    screen
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}
