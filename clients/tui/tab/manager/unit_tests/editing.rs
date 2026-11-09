use super::fixtures::*;
use crate::{Action, Manager};
use crossterm::event::KeyCode;
use yolo_tui_tab::MAX_TAB_NAME_BYTES;

#[test]
fn unicode_name_trim_and_backspace_preserve_valid_utf8() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    manager.create();
    paste(&mut manager, &catalog, PADDED_UNICODE_NAME);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Create(UNICODE_NAME.into()))
    );
    manager.create();
    paste(&mut manager, &catalog, WIDE);
    press(&mut manager, &catalog, KeyCode::Backspace);
    press(&mut manager, &catalog, KeyCode::Backspace);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Create(String::new()))
    );
}

#[test]
fn paste_discards_controls_and_enforces_byte_limit_without_splitting_unicode() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    manager.create();
    paste(&mut manager, &catalog, CONTROL_TEXT);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Create(CLEAN_TEXT.into()))
    );
    manager.create();
    let prefix = ASCII.repeat(MAX_TAB_NAME_BYTES - WIDE.len());
    paste(&mut manager, &catalog, &prefix);
    paste(&mut manager, &catalog, WIDE);
    paste(&mut manager, &catalog, ASCII);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Create(format!("{prefix}{WIDE}")))
    );
    manager.create();
    let full = ASCII.repeat(MAX_TAB_NAME_BYTES);
    paste(&mut manager, &catalog, &full);
    paste(&mut manager, &catalog, WIDE);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Create(full))
    );
}

#[test]
fn direct_rename_shortcut_prefills_existing_name_and_paste_replaces_it_without_mutating_catalog() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    manager.rename(&catalog);
    paste(&mut manager, &catalog, WIDE);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Rename(catalog.active().id, WIDE.into()))
    );
}
