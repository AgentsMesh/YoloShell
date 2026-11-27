use super::fixture::*;
use crate::{
    fields::FieldId,
    model::{Action, Page},
};
use crossterm::event::KeyCode;
#[path = "../test_support/ssh_constants.rs"]
mod constants;
use constants::*;

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
    field(&mut manager, FieldId::Ssh, PROFILE);
    field(&mut manager, FieldId::Socket, SOCKET);
    save(&mut manager);
    assert!(manager.form.is_none() && manager.dirty() && !manager.pending());
    let peer = manager.draft.as_ref().unwrap().peer(pin.hub()).unwrap();
    assert!(peer.consume && peer.exports.is_empty());
    let dial = peer.dial.as_ref().unwrap();
    assert_eq!(dial.ssh_target(), Some(PROFILE));
    assert_eq!(dial.socket().to_str(), Some(SOCKET));
    activate(&mut manager, Action::Page(Page::Peer(pin.hub())));
    activate(&mut manager, Action::EditPeer(pin.hub()));
    field(&mut manager, FieldId::Key, "ff");
    field(&mut manager, FieldId::Ssh, "");
    field(&mut manager, FieldId::Socket, "");
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
        (FieldId::Ssh, "bad host"),
        (FieldId::Socket, "relative.sock"),
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
        field(&mut manager, FieldId::Ssh, PROFILE);
        field(&mut manager, FieldId::Socket, SOCKET);
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
fn peer_socket_edits_reject_partial_pairs_and_reload_does_not_silently_discard_drafts() {
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    activate(&mut manager, Action::Page(Page::Peer(pin.hub())));
    activate(&mut manager, Action::EditPeer(pin.hub()));
    let before = manager.draft.clone();
    field(&mut manager, FieldId::Ssh, PROFILE);
    save(&mut manager);
    assert_eq!(manager.draft, before);
    field(&mut manager, FieldId::Socket, SOCKET);
    save(&mut manager);
    assert!(manager.dirty());
    manager.page(crate::model::Page::Root);
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
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    manager.base.as_mut().unwrap().state.configuration = manager.draft.clone().unwrap();
    activate(&mut manager, Action::Page(Page::Peer(pin.hub())));
    activate(&mut manager, Action::EditPeer(pin.hub()));
    save(&mut manager);
    assert!(!manager.dirty());
    assert_eq!(manager.status, crate::constants::READY);
    manager.base.as_mut().unwrap().applied = false;
    activate(&mut manager, Action::EditPeer(pin.hub()));
    save(&mut manager);
    assert!(!manager.dirty());
    assert_eq!(manager.status, crate::constants::INACTIVE);
}
