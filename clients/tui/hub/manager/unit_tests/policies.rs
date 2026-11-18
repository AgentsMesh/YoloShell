use super::fixture::*;
use crate::{
    fields::FieldId,
    model::{Action, Page},
};
use crossterm::event::KeyCode;
use yolo_hub_routing::Right;

#[test]
fn new_exports_start_observe_only_and_each_right_is_an_independent_explicit_choice() {
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    manager.page(Page::Exports(pin.hub()));
    activate(&mut manager, Action::AddExport(pin.hub()));
    save(&mut manager);
    let origin = manager.entry;
    let policy = manager
        .draft
        .as_ref()
        .unwrap()
        .export(pin.hub(), origin)
        .unwrap();
    assert_eq!(policy.rights, [Right::Observe].into_iter().collect());
    activate(&mut manager, Action::EditExport(pin.hub(), origin));
    for id in [
        FieldId::ReadTerminal,
        FieldId::WriteTerminal,
        FieldId::Lifecycle,
        FieldId::FileRead,
        FieldId::AgentControl,
    ] {
        let form = manager.form.as_mut().unwrap();
        form.focused = form.fields.iter().position(|field| field.id == id).unwrap();
        key(&mut manager, KeyCode::Char(' '));
    }
    let form = manager.form.as_mut().unwrap();
    form.focused = form
        .fields
        .iter()
        .position(|field| field.id == FieldId::Reexports)
        .unwrap();
    key(&mut manager, KeyCode::End);
    key(&mut manager, KeyCode::Right);
    save(&mut manager);
    let policy = manager
        .draft
        .as_ref()
        .unwrap()
        .export(pin.hub(), origin)
        .unwrap();
    assert_eq!(
        policy.remaining_reexports,
        yolo_hub_delegation::MAX_DELEGATION_DEPTH
    );
    assert!(
        policy.rights.allows(
            &[
                Right::Observe,
                Right::TerminalRead,
                Right::TerminalWrite,
                Right::Lifecycle,
                Right::FileRead,
                Right::AgentControl
            ]
            .into_iter()
            .collect()
        )
    );
    activate(&mut manager, Action::EditExport(pin.hub(), origin));
    let form = manager.form.as_mut().unwrap();
    form.focused = form.rows() - 1;
    key(&mut manager, KeyCode::Enter);
    key(&mut manager, KeyCode::Tab);
    key(&mut manager, KeyCode::Enter);
    assert!(
        manager
            .draft
            .as_ref()
            .unwrap()
            .export(pin.hub(), origin)
            .is_none()
    );
}

#[test]
fn trusted_origin_removal_prunes_dependent_exports_but_keeps_an_independent_peer_pin() {
    let mut manager = manager();
    let relay = peer_identity();
    let origin = peer_identity();
    add_peer(&mut manager, relay);
    manager.page(Page::Origins);
    activate(&mut manager, Action::AddOrigin);
    field(&mut manager, FieldId::Hub, &origin.hub().to_string());
    // Every Hub key must be distinct, including origin pins.
    let origin = yolo_hub_delegation::PeerIdentity::new(
        origin.hub(),
        [3; yolo_hub_delegation::ED25519_PUBLIC_KEY_BYTES],
    )
    .unwrap();
    field(
        &mut manager,
        FieldId::Key,
        &crate::wire_fields::key_text(origin),
    );
    save(&mut manager);
    manager.page(Page::Exports(relay.hub()));
    activate(&mut manager, Action::AddExport(relay.hub()));
    field(&mut manager, FieldId::Origin, &origin.hub().to_string());
    save(&mut manager);
    assert!(manager.form.is_none());
    let mut independent = manager.draft.clone().unwrap();
    independent.peers.push(yolo_hub_federation_settings::Peer {
        identity: origin,
        dial: None,
        consume: true,
        exports: vec![],
    });
    manager.draft = Some(independent.clone());
    manager.remove(Action::RemoveOrigin(origin.hub()));
    assert!(
        manager
            .draft
            .as_ref()
            .unwrap()
            .export(relay.hub(), origin.hub())
            .is_some()
    );
    manager.draft = Some(independent);
    manager.remove(Action::RemovePeer(origin.hub()));
    assert!(
        manager
            .draft
            .as_ref()
            .unwrap()
            .export(relay.hub(), origin.hub())
            .is_some()
    );
    manager.remove(Action::RemoveOrigin(origin.hub()));
    assert!(
        manager
            .draft
            .as_ref()
            .unwrap()
            .export(relay.hub(), origin.hub())
            .is_none()
    );
    assert!(manager.draft.as_ref().unwrap().peer(relay.hub()).is_some());
}

#[test]
fn unknown_export_origins_and_repeated_trust_entries_cannot_enter_the_draft() {
    let mut manager = manager();
    let relay = peer_identity();
    add_peer(&mut manager, relay);
    manager.page(Page::Exports(relay.hub()));
    activate(&mut manager, Action::AddExport(relay.hub()));
    field(
        &mut manager,
        FieldId::Origin,
        &peer_identity().hub().to_string(),
    );
    let before = manager.draft.clone();
    save(&mut manager);
    assert_eq!(manager.draft, before);
    key(&mut manager, KeyCode::Esc);
    manager.page(Page::Origins);
    activate(&mut manager, Action::AddOrigin);
    field(&mut manager, FieldId::Hub, &relay.hub().to_string());
    field(
        &mut manager,
        FieldId::Key,
        &crate::wire_fields::key_text(relay),
    );
    save(&mut manager);
    assert!(manager.form.is_none());
    activate(&mut manager, Action::AddOrigin);
    field(&mut manager, FieldId::Hub, &relay.hub().to_string());
    field(
        &mut manager,
        FieldId::Key,
        &crate::wire_fields::key_text(relay),
    );
    let before = manager.draft.clone();
    save(&mut manager);
    assert_eq!(manager.draft, before);
}
