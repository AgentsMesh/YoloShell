use super::fixture::*;
use crate::{
    Effect,
    fields::FieldId,
    form::Form,
    model::{Action, Page},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[test]
fn enter_saves_a_complete_peer_form_and_cancel_preserves_the_previous_draft() {
    let mut manager = manager();
    let pin = peer_identity();
    manager.page(Page::Peers);
    activate(&mut manager, Action::AddPeer);
    field(&mut manager, FieldId::Hub, &pin.hub().to_string());
    field(
        &mut manager,
        FieldId::Key,
        &crate::wire_fields::key_text(pin),
    );
    field(&mut manager, FieldId::Profile, "test-machine");
    field(
        &mut manager,
        FieldId::Socket,
        "/run/user/1000/yolo/hub/peer.sock",
    );
    save(&mut manager);
    assert!(manager.form.is_none() && manager.dirty());
    let before = manager.draft.clone();
    manager.page(Page::Peer(pin.hub()));
    activate(&mut manager, Action::EditPeer(pin.hub()));
    field(&mut manager, FieldId::Profile, "changed");
    key(&mut manager, KeyCode::Esc);
    assert_eq!(manager.draft, before);
}

#[test]
fn navigation_checks_unknown_results_without_resending_mutations_and_allows_safe_close() {
    let mut manager = manager();
    edit(&mut manager);
    let request = manager.proposal().unwrap();
    assert!(key(&mut manager, KeyCode::Enter).is_none());
    assert!(matches!(
        key(&mut manager, KeyCode::Esc),
        Some(Effect::Close)
    ));
    manager.complete(
        request.token,
        Err(yolo_hub_management_client::ManagementError::OutcomeUnknown),
    );
    assert!(
        manager
            .items()
            .iter()
            .any(|item| item.action == Action::Check)
    );
    let Some(Effect::Request(query)) = activate(&mut manager, Action::Check) else {
        panic!()
    };
    assert_eq!(
        query.command,
        yolo_hub_rpc_protocol::Command::GetConfiguration {}
    );
    manager.not_sent(query.token);
    key(&mut manager, KeyCode::Home);
    key(&mut manager, KeyCode::Down);
    key(&mut manager, KeyCode::BackTab);
    activate(&mut manager, Action::Page(Page::Peers));
    key(&mut manager, KeyCode::Esc);
    activate(&mut manager, Action::Close);
    key(&mut manager, KeyCode::Char('z'));
    key(&mut manager, KeyCode::Esc);
    assert!(manager.handle(Event::FocusLost).is_none());
    assert!(manager.handle(Event::Paste("unused".into())).is_none());
    assert!(key(&mut manager, KeyCode::F(1)).is_none());
    assert!(key(&mut manager, KeyCode::Char('q')).is_none());
}

#[test]
fn clear_navigation_and_unhandled_keys_are_safe_at_field_and_command_boundaries() {
    let mut form = Form::peer(None);
    for code in [
        KeyCode::Right,
        KeyCode::Left,
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::BackTab,
        KeyCode::Tab,
    ] {
        form.key(KeyEvent::new(code, KeyModifiers::NONE));
    }
    form.focused = form.fields.len();
    assert!(!form.paste("ignored"));
    form.key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    form.focused = 0;
    form.paste("abc");
    form.key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    form.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    form.key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
    form.key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
    form.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert!(form.fields[0].display().is_empty());
    let mut export = Form::export(peer_identity().hub(), peer_identity().hub(), None);
    export.focused = export.fields.len() - 1;
    export.key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
}

#[test]
fn errors_and_unapplied_receipts_keep_the_submitted_draft_blocked() {
    let mut manager = manager();
    edit(&mut manager);
    let request = manager.proposal().unwrap();
    let mut reply = manager.base.clone().unwrap();
    let yolo_hub_rpc_protocol::Command::Configure { change } = request.command else {
        panic!()
    };
    reply.state.revision += 1;
    reply.state.last_change = change.request_id;
    reply.state.configuration = change.configuration;
    reply.applied = false;
    manager.complete(request.token, Ok(reply));
    assert!(manager.blocked && manager.unresolved.is_some());
    let query = manager.query(true);
    manager.complete(
        query.token,
        Err(yolo_hub_management_client::ManagementError::Unavailable),
    );
    assert!(manager.blocked && manager.unresolved.is_some());
    let query = manager.query(false);
    let mut malformed = manager.base.clone().unwrap();
    malformed.state.schema_version = 0;
    manager.complete(query.token, Ok(malformed));
    assert!(manager.blocked);
}
