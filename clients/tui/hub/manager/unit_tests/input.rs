use super::fixture::*;
use crate::{
    fields::{FieldId, Value},
    model::{Action, Page},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[test]
fn unicode_text_editing_and_paste_limits_preserve_utf8_and_reject_terminal_controls() {
    let mut manager = manager();
    activate(&mut manager, Action::Listener);
    field(&mut manager, FieldId::Listen, "a\u{e9}z");
    for code in [
        KeyCode::Left,
        KeyCode::Backspace,
        KeyCode::Home,
        KeyCode::Delete,
        KeyCode::End,
        KeyCode::Char('x'),
    ] {
        key(&mut manager, code);
    }
    let form = manager.form.as_ref().unwrap();
    assert_eq!(form.fields[0].display(), "zx");
    manager.handle(Event::Paste("\u{1b}[31m".into()));
    assert_eq!(
        manager.form.as_ref().unwrap().fields[0].display(),
        "zx[31m",
        "terminal controls are filtered, printable paste text is kept"
    );
    manager.handle(Event::Paste(
        "a".repeat(yolo_hub_federation_settings::MAX_HOST_BYTES),
    ));
    let display = manager.form.as_ref().unwrap().fields[0].display();
    assert_eq!(display.len(), yolo_hub_federation_settings::MAX_HOST_BYTES);
    assert!(display.starts_with("zx[31m"));
    let release = KeyEvent {
        code: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::NONE,
    };
    manager.handle(Event::Key(release));
    assert_eq!(manager.form.as_ref().unwrap().fields[0].display(), display);
}

#[test]
fn form_navigation_skips_immutable_identity_fields_and_cancels_without_saving() {
    let mut manager = manager();
    let peer = peer_identity();
    add_peer(&mut manager, peer);
    manager.page(Page::Peer(peer.hub()));
    activate(&mut manager, Action::EditPeer(peer.hub()));
    let form = manager.form.as_mut().unwrap();
    assert_eq!(form.fields[form.focused].id, FieldId::Host);
    form.move_focus(false);
    assert!(form.focused >= form.fields.len());
    form.move_focus(true);
    assert_eq!(form.fields[form.focused].id, FieldId::Host);
    let before = manager.draft.clone();
    field(&mut manager, FieldId::Host, "changed.example");
    key(&mut manager, KeyCode::Esc);
    assert_eq!(manager.draft, before);
}

#[test]
fn keyboard_navigation_and_confirmed_close_do_not_emit_unintended_mutations() {
    let mut manager = manager();
    for code in [
        KeyCode::End,
        KeyCode::Up,
        KeyCode::Home,
        KeyCode::Down,
        KeyCode::BackTab,
    ] {
        assert!(key(&mut manager, code).is_none());
    }
    edit(&mut manager);
    activate(&mut manager, Action::Close);
    key(&mut manager, KeyCode::Enter);
    assert!(manager.dirty());
    activate(&mut manager, Action::Close);
    key(&mut manager, KeyCode::BackTab);
    assert!(matches!(
        key(&mut manager, KeyCode::Enter),
        Some(crate::Effect::Close)
    ));
}

#[test]
fn stepper_stays_bounded_and_disabled_fields_do_not_accept_paste() {
    let mut form = crate::form::Form::export(peer_identity().hub(), peer_identity().hub(), None);
    form.focused = form
        .fields
        .iter()
        .position(|field| field.id == FieldId::Reexports)
        .unwrap();
    for code in [
        KeyCode::Left,
        KeyCode::Char('-'),
        KeyCode::End,
        KeyCode::Right,
        KeyCode::Char('+'),
        KeyCode::Home,
    ] {
        form.key(KeyEvent::new(code, KeyModifiers::NONE));
    }
    assert!(matches!(form.fields[form.focused].value, Value::Stepper(0)));
    assert!(!form.paste("9"));
    form.focused = form
        .fields
        .iter()
        .position(|field| field.id == FieldId::Observe)
        .unwrap();
    form.key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert_eq!(
        form.fields[form.focused].display(),
        crate::constants::CHECKED
    );
}
