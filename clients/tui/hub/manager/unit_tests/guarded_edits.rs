use super::fixture::*;
use crate::{
    fields::{Field, FieldId, Value},
    form::Form,
    model::{Action, Page},
    wire_fields,
};
use yolo_hub_identity::HubId;

#[test]
fn stale_identity_edits_cannot_redirect_an_existing_peer_or_export() {
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    let draft = manager.draft.clone().unwrap();
    let local = manager.base.as_ref().unwrap().state.identity;
    let mut form = Form::peer(draft.peer(pin.hub()));
    form.fields[0] = Field::text(FieldId::Hub, HubId::new().to_string(), false);
    assert!(form.commit(&draft, local).is_err());
    let policy = yolo_hub_federation_settings::ExportPolicy {
        origin: local.hub(),
        rights: [yolo_hub_routing::Right::Observe].into_iter().collect(),
        remaining_reexports: 0,
    };
    let mut form = Form::export(pin.hub(), local.hub(), Some(&policy));
    form.fields[0] = Field::text(FieldId::Origin, pin.hub().to_string(), false);
    assert!(form.commit(&draft, local).is_err());
}

#[test]
fn malformed_fields_and_public_keys_return_errors_instead_of_constructing_authority() {
    let pin = peer_identity();
    for key in [
        "00".repeat(32),
        "xx".repeat(32),
        "\u{e9}".repeat(32),
        "ff".repeat(31),
    ] {
        let fields = [
            Field::text(FieldId::Hub, pin.hub().to_string(), false),
            Field::text(FieldId::Key, key, false),
        ];
        assert!(wire_fields::identity(&fields).is_err());
    }
    let fields = [Field {
        id: FieldId::Hub,
        value: Value::Toggle(true),
        read_only: false,
    }];
    assert!(wire_fields::text(&fields, FieldId::Hub).is_err());
    assert!(wire_fields::flag(&fields, FieldId::Import).is_err());
    assert!(wire_fields::number(&fields, FieldId::Reexports).is_err());
}

#[test]
fn incomplete_and_invalid_drafts_cannot_emit_a_change_or_be_partially_rewritten() {
    let snapshot = snapshot();
    let (mut loading, _) = crate::Manager::open(snapshot.state.identity.hub());
    loading.remove(Action::RemovePeer(peer_identity().hub()));
    loading.save_form();
    loading.draft = Some(Default::default());
    loading.remove(Action::RemoveOrigin(peer_identity().hub()));
    assert!(loading.base.is_none());
    let mut manager = manager();
    let local = manager.base.as_ref().unwrap().state.identity;
    manager.draft.as_mut().unwrap().trusted_origins.push(local);
    let invalid = manager.draft.clone();
    manager.remove(Action::RemovePeer(peer_identity().hub()));
    assert_eq!(manager.draft, invalid);
    assert!(manager.proposal().is_none());
    manager.remove(Action::Reload);
    assert_eq!(manager.draft, invalid);
}

#[test]
fn removing_an_export_is_narrow_and_does_not_remove_the_connection() {
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    manager.page(Page::Exports(pin.hub()));
    activate(&mut manager, Action::AddExport(pin.hub()));
    save(&mut manager);
    manager.remove(Action::RemoveExport(pin.hub(), manager.entry));
    assert!(
        manager
            .draft
            .as_ref()
            .unwrap()
            .peer(pin.hub())
            .unwrap()
            .exports
            .is_empty()
    );
}
