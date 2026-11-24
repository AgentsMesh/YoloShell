use super::fixtures::*;
use crate::{Action, Manager};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[test]
fn rename_can_replace_a_prefilled_name_and_edit_in_the_middle() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    manager.rename(&catalog);
    manager.handle(
        Event::Key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)),
        &catalog,
    );
    paste(&mut manager, &catalog, "Build中");
    press(&mut manager, &catalog, KeyCode::Left);
    paste(&mut manager, &catalog, "-工作");
    press(&mut manager, &catalog, KeyCode::Delete);
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Rename(catalog.active().id, "Build-工作".into()))
    );
}

#[test]
fn typing_after_rename_replaces_the_prefilled_name() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    manager.rename(&catalog);
    paste(&mut manager, &catalog, "Build");
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Rename(catalog.active().id, "Build".into()))
    );
}

#[test]
fn a_pasted_newline_cannot_submit_a_tab_or_escape_the_form() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    manager.create();
    paste(&mut manager, &catalog, "New\n\r\x1bTab");
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Create("NewTab".into()))
    );
}
