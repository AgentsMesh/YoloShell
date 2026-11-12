use super::fixtures::*;
use crate::{Action, Manager, constants::*};
use crossterm::event::KeyCode;

#[test]
fn arrows_and_vi_keys_select_real_entries_without_wrapping_past_list_ends() {
    let catalog = catalog();
    for (next, previous) in [
        (KeyCode::Down, KeyCode::Up),
        (KeyCode::Char(NEXT_KEY), KeyCode::Char(PREVIOUS_KEY)),
    ] {
        let mut manager = Manager::new(&catalog);
        for _ in usize::default()..MAX_NAVIGATION_ATTEMPTS {
            press(&mut manager, &catalog, next);
        }
        assert_eq!(
            press(&mut manager, &catalog, KeyCode::Enter),
            Some(Action::Select(catalog.entries().last().unwrap().id))
        );
        for _ in usize::default()..MAX_NAVIGATION_ATTEMPTS {
            press(&mut manager, &catalog, previous);
        }
        assert_eq!(
            press(&mut manager, &catalog, KeyCode::Enter),
            Some(Action::Select(catalog.entries().first().unwrap().id))
        );
    }
}

#[test]
fn a_catalog_replacement_cannot_make_rename_target_an_unrelated_workspace() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    let replacement = yolo_tui_tab::TabCatalog::default();
    manager.rename(&replacement);
    paste(&mut manager, &replacement, NEW_NAME);
    assert_eq!(
        press(&mut manager, &replacement, KeyCode::Enter),
        Some(Action::Select(catalog.active().id))
    );
    press(&mut manager, &replacement, KeyCode::Down);
    assert_eq!(
        press(&mut manager, &replacement, KeyCode::Enter),
        Some(Action::Select(replacement.active().id))
    );
}
