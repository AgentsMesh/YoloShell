use crate::{Action, Manager};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use yolo_tui_tab::TabCatalog;

#[path = "constants.rs"]
mod constants;
pub use constants::*;

pub fn catalog() -> TabCatalog {
    let mut catalog = TabCatalog::default();
    let first = catalog.active().id;
    catalog.create(SECOND_NAME).unwrap();
    catalog.create(THIRD_NAME).unwrap();
    catalog.select(first).unwrap();
    catalog
}

pub fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
pub fn press(manager: &mut Manager, catalog: &TabCatalog, code: KeyCode) -> Option<Action> {
    manager.handle(key(code), catalog)
}
pub fn paste(manager: &mut Manager, catalog: &TabCatalog, text: &str) {
    assert_eq!(manager.handle(Event::Paste(text.into()), catalog), None);
}

pub fn rendered(manager: &Manager, catalog: &TabCatalog, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| crate::render(frame, manager, catalog, frame.area()))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

pub fn empty_area(manager: &Manager, catalog: &TabCatalog) {
    let mut terminal = Terminal::new(TestBackend::new(FRAME_WIDTH, FRAME_HEIGHT)).unwrap();
    terminal
        .draw(|frame| crate::render(frame, manager, catalog, Rect::default()))
        .unwrap();
}
