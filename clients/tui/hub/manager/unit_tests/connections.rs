use super::fixture::*;
use crate::{
    fields::FieldId,
    model::{Action, Page},
};
use crossterm::event::KeyCode;

#[test]
fn peer_creation_editing_and_dial_direction_are_draft_only() {
    let mut manager = manager();
    let pin = peer_identity();
    activate(&mut manager, Action::Page(Page::Peers));
    activate(&mut manager, Action::AddPeer);
    field(&mut manager, FieldId::Hub, &pin.hub().to_string());
    field(
        &mut manager,
        FieldId::Key,
        &crate::wire_fields::key_text(pin),
    );
    field(&mut manager, FieldId::Host, "localhost");
    field(&mut manager, FieldId::Port, "2222");
    save(&mut manager);
    assert!(manager.form.is_none() && manager.dirty() && !manager.pending());
    let peer = manager.draft.as_ref().unwrap().peer(pin.hub()).unwrap();
    assert!(peer.consume && peer.exports.is_empty());
    assert_eq!(peer.dial.as_ref().unwrap().host, "localhost");
    activate(&mut manager, Action::Page(Page::Peer(pin.hub())));
    activate(&mut manager, Action::EditPeer(pin.hub()));
    field(&mut manager, FieldId::Key, "ff");
    field(&mut manager, FieldId::Host, "");
    field(&mut manager, FieldId::Port, "");
    let form = manager.form.as_mut().unwrap();
    form.focused = form
        .fields
        .iter()
        .position(|field| field.id == FieldId::Import)
        .unwrap();
    key(&mut manager, KeyCode::Char(' '));
    save(&mut manager);
    let peer = manager.draft.as_ref().unwrap().peer(pin.hub()).unwrap();
    assert_eq!(peer.identity, pin);
    assert!(!peer.consume && peer.dial.is_none());
    assert!(
        manager
            .base
            .as_ref()
            .unwrap()
            .state
            .configuration
            .peers
            .is_empty()
    );
}

#[test]
fn malformed_identity_duplicate_peer_and_invalid_dial_are_atomic_rejections() {
    for (id, value) in [
        (FieldId::Hub, "invalid"),
        (FieldId::Key, "00"),
        (FieldId::Host, "bad host"),
        (FieldId::Port, "0"),
    ] {
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
        field(&mut manager, FieldId::Host, "localhost");
        field(&mut manager, FieldId::Port, "2222");
        field(&mut manager, id, value);
        save(&mut manager);
        assert!(manager.form.is_some() && !manager.dirty());
    }
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    let original = manager.draft.clone();
    activate(&mut manager, Action::AddPeer);
    field(&mut manager, FieldId::Hub, &pin.hub().to_string());
    field(
        &mut manager,
        FieldId::Key,
        &crate::wire_fields::key_text(pin),
    );
    save(&mut manager);
    assert!(manager.form.is_some());
    assert_eq!(manager.draft, original);
}

#[test]
fn removing_peer_requires_confirmation_and_preserves_runtime_independent_configuration() {
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    activate(&mut manager, Action::Page(Page::Peer(pin.hub())));
    activate(&mut manager, Action::RemovePeer(pin.hub()));
    key(&mut manager, KeyCode::Enter);
    assert!(manager.draft.as_ref().unwrap().peer(pin.hub()).is_some());
    activate(&mut manager, Action::RemovePeer(pin.hub()));
    key(&mut manager, KeyCode::Down);
    key(&mut manager, KeyCode::Enter);
    assert!(manager.draft.as_ref().unwrap().peers.is_empty());
    assert!(!manager.pending());
}

#[test]
fn listener_validates_addresses_and_reload_does_not_silently_discard_drafts() {
    let mut manager = manager();
    activate(&mut manager, Action::Listener);
    field(&mut manager, FieldId::Listen, "bad-address");
    save(&mut manager);
    assert!(!manager.dirty());
    field(&mut manager, FieldId::Listen, "[::1]:0");
    save(&mut manager);
    assert!(manager.dirty());
    activate(&mut manager, Action::Reload);
    key(&mut manager, KeyCode::Esc);
    assert!(manager.dirty() && !manager.pending());
    activate(&mut manager, Action::Reload);
    key(&mut manager, KeyCode::Tab);
    assert!(matches!(
        key(&mut manager, KeyCode::Enter),
        Some(crate::Effect::Request(_))
    ));
}

#[test]
fn saving_an_unchanged_form_does_not_claim_unsaved_configuration() {
    let mut manager = manager();
    activate(&mut manager, Action::Listener);
    save(&mut manager);
    assert!(!manager.dirty());
    assert_eq!(manager.status, crate::constants::READY);
    manager.base.as_mut().unwrap().applied = false;
    activate(&mut manager, Action::Listener);
    save(&mut manager);
    assert!(!manager.dirty());
    assert_eq!(manager.status, crate::constants::INACTIVE);
}
