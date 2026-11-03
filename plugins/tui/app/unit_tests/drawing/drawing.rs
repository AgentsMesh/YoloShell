use super::fixtures::*;
use crate::{Mode, draw::draw};
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn document_mode_renders_the_open_document_in_the_content_area() {
    let mut state = crate::client_capability_support::state();
    state.mode = Mode::Document;
    assert!(state
        .document
        .open("Report".into(), "document body".into(), &mut state.tabs));
    let (text, presentation) = super::drawing_support::render(&state, 80, 24);
    assert!(text.contains("Report"), "{text}");
    assert!(text.contains("document body"), "{text}");
    assert!(presentation.inventory.is_none());
    assert_eq!(presentation.bounds.width, 80);
}

#[test]
fn every_mode_renders_without_overflow_at_compact_and_desktop_sizes() {
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![session()]);
    for (width, height) in [(1, 1), (20, 5), (80, 24), (160, 48)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        for mode in [
            Mode::Inventory,
            Mode::Terminal,
            Mode::Search,
            Mode::Directory,
            Mode::ConfirmTerminate,
            Mode::ConfirmArchive,
            Mode::HubConfiguration,
        ] {
            state.mode = mode;
            terminal
                .draw(|frame| {
                    draw(frame, &state);
                })
                .unwrap();
        }
        state.replace_terminals(Vec::new());
        state.mode = Mode::Inventory;
        terminal
            .draw(|frame| {
                draw(frame, &state);
            })
            .unwrap();
    }
}
