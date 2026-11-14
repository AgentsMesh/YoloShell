use super::fixtures::*;
use crate::{Action, Manager, constants::*};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

#[test]
fn only_press_events_and_unmodified_or_shift_keys_can_edit_confirm_or_close() {
    let catalog = catalog();
    for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
        let mut manager = Manager::new(&catalog);
        manager.create();
        for code in [
            KeyCode::Enter,
            KeyCode::Esc,
            KeyCode::Char(CREATE_KEY),
            KeyCode::Backspace,
        ] {
            let event = Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind));
            assert_eq!(manager.handle(event, &catalog), None);
        }
        assert_eq!(
            press(&mut manager, &catalog, KeyCode::Enter),
            Some(Action::Create(String::new()))
        );
    }
    for modifier in [
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::SUPER,
        KeyModifiers::HYPER,
        KeyModifiers::META,
    ] {
        let mut manager = Manager::new(&catalog);
        manager.create();
        for code in [
            KeyCode::Enter,
            KeyCode::Esc,
            KeyCode::Char(SHIFT_CHARACTER),
            KeyCode::Backspace,
        ] {
            assert_eq!(
                manager.handle(Event::Key(KeyEvent::new(code, modifier)), &catalog),
                None
            );
        }
        assert_eq!(
            manager.handle(
                Event::Key(KeyEvent::new(
                    KeyCode::Char(SHIFT_CHARACTER),
                    KeyModifiers::SHIFT
                )),
                &catalog
            ),
            None
        );
        assert_eq!(
            press(&mut manager, &catalog, KeyCode::Enter),
            Some(Action::Create(SHIFT_CHARACTER.to_string()))
        );
    }
}

#[test]
fn resize_focus_mouse_and_paste_on_inventory_never_submit_an_operation() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    for event in [
        Event::Resize(FRAME_WIDTH, FRAME_HEIGHT),
        Event::FocusGained,
        Event::FocusLost,
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: u16::default(),
            row: u16::default(),
            modifiers: KeyModifiers::NONE,
        }),
        Event::Paste(NEW_NAME.into()),
    ] {
        assert_eq!(manager.handle(event, &catalog), None);
    }
    assert_eq!(
        press(&mut manager, &catalog, KeyCode::Enter),
        Some(Action::Select(catalog.active().id))
    );
}
