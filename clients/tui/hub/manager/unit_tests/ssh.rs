use super::fixture::*;
use crate::{
    fields::{FieldId, Value},
    form::Form,
};
#[path = "../test_support/ssh_constants.rs"]
mod constants;
use constants::*;

#[test]
fn editing_imports_preserves_the_ssh_socket_dial_until_explicitly_changed() {
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
        yolo_hub_federation_settings::PeerAddress::ssh(PROFILE, SOCKET.into()).unwrap(),
    );
    let mut form = Form::peer(Some(peer));
    let original = peer.dial.clone();
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
        .find(|field| field.id == FieldId::Ssh)
        .unwrap()
        .value = Value::Text {
        text: CHANGED_PROFILE.into(),
        cursor: Default::default(),
    };
    let changed = form.commit(&draft, local).unwrap();
    assert_eq!(
        changed
            .peer(pin.hub())
            .unwrap()
            .dial
            .as_ref()
            .unwrap()
            .ssh_target(),
        Some(CHANGED_PROFILE)
    );
    form.fields
        .iter_mut()
        .find(|field| field.id == FieldId::Socket)
        .unwrap()
        .value = Value::Text {
        text: "relative.sock".into(),
        cursor: Default::default(),
    };
    assert!(form.commit(&draft, local).is_err());
}
