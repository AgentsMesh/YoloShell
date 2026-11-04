use super::fixture::*;
use crate::{
    fields::{FieldId, Value},
    form::Form,
};
use yolo_hub_federation_settings::{PeerAddress, SshProfileRef};
#[path = "../test_support/ssh_constants.rs"]
mod constants;
use constants::*;

#[test]
fn editing_imports_keeps_ssh_reference_and_rejects_endpoint_tampering() {
    let mut manager = manager();
    let pin = peer_identity();
    add_peer(&mut manager, pin);
    let mut draft = manager.draft.clone().unwrap();
    let peer = draft
        .peers
        .iter_mut()
        .find(|peer| peer.identity == pin)
        .unwrap();
    peer.dial = Some(
        PeerAddress::ssh(
            SshProfileRef::new(PROFILE_ID, DIGEST_CHARACTER.repeat(DIGEST_BYTES)).unwrap(),
            ENDPOINT.parse().unwrap(),
        )
        .unwrap(),
    );
    let mut form = Form::peer(Some(peer));
    let original = peer.dial.clone();
    for field in [FieldId::Host, FieldId::Port] {
        assert!(
            form.fields
                .iter()
                .find(|entry| entry.id == field)
                .unwrap()
                .read_only
        );
    }
    form.fields
        .iter_mut()
        .find(|field| field.id == FieldId::Import)
        .unwrap()
        .value = Value::Toggle(false);
    let local = manager.base.as_ref().unwrap().state.identity;
    let changed = form.commit(&draft, local).unwrap();
    assert_eq!(changed.peer(pin.hub()).unwrap().dial, original);
    assert!(!changed.peer(pin.hub()).unwrap().consume);
    form.fields
        .iter_mut()
        .find(|field| field.id == FieldId::Host)
        .unwrap()
        .value = Value::Text {
        text: OTHER_HOST.into(),
        cursor: Default::default(),
    };
    assert!(form.commit(&draft, local).is_err());
}
