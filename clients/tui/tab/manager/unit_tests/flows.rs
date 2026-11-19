use super::fixtures::*;
use crate::{Action, Manager, constants::*};
use crossterm::event::KeyCode;

#[test]
fn creating_then_renaming_preserves_the_selected_workspace_identity_and_layout() {
    let mut catalog = catalog();
    let previous = catalog.clone();
    let mut manager = Manager::new(&catalog);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Char(CREATE_KEY)),
        None
    );
    paste(&mut manager, &catalog, NEW_NAME);
    assert_eq!(catalog, previous);
    let Some(Action::Create(name)) = press(&mut manager, &catalog, KeyCode::Enter) else {
        panic!()
    };
    let created = catalog.create(&name).unwrap();
    let layout = catalog.active().view.clone();
    let mut manager = Manager::new(&catalog);
    press(&mut manager, &catalog, KeyCode::Char(RENAME_KEY));
    for _ in NEW_NAME.chars() {
        press(&mut manager, &catalog, KeyCode::Backspace);
    }
    paste(&mut manager, &catalog, RENAMED_NAME);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Rename(created, RENAMED_NAME.into()))
    );
    catalog.rename(created, RENAMED_NAME).unwrap();
    assert_eq!(catalog.active().id, created);
    assert_eq!(catalog.active().view, layout);
}

#[test]
fn remove_requires_enter_and_escape_cancels_without_changing_any_workspace() {
    let catalog = catalog();
    let previous = catalog.clone();
    let mut manager = Manager::new(&catalog);
    press(&mut manager, &catalog, KeyCode::Down);
    let target = catalog.entries()[STEP].id;
    press(&mut manager, &catalog, KeyCode::Char(REMOVE_KEY));
    paste(&mut manager, &catalog, NEW_NAME);
    for key in [KeyCode::Backspace, KeyCode::Char(CREATE_KEY), KeyCode::Up] {
        assert_eq!(press(&mut manager, &catalog, key), None);
    }
    assert_eq!(press(&mut manager, &catalog, KeyCode::Esc), None);
    assert_eq!(catalog, previous);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Select(target))
    );
    press(&mut manager, &catalog, KeyCode::Char(REMOVE_KEY));
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Remove(target))
    );
    assert_eq!(catalog, previous);
}

#[test]
fn validation_error_keeps_editable_form_and_cancel_returns_to_inventory() {
    let mut catalog = catalog();
    let mut manager = Manager::new(&catalog);
    manager.create();
    let Some(Action::Create(name)) = press(&mut manager, &catalog, KeyCode::Enter) else {
        panic!()
    };
    assert!(catalog.create(&name).is_err());
    manager.error(ERROR_MESSAGE.into());
    assert!(rendered(&manager, &catalog, FRAME_WIDTH, FRAME_HEIGHT).contains(ERROR_MESSAGE));
    paste(&mut manager, &catalog, NEW_NAME);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Create(NEW_NAME.into()))
    );
    assert_eq!(press(&mut manager, &catalog, KeyCode::Esc), None);
    assert!(!rendered(&manager, &catalog, FRAME_WIDTH, FRAME_HEIGHT).contains(ERROR_MESSAGE));
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Esc),
        Some(Action::Close)
    );
}
