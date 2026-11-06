use crate::*;
use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};

pub fn snapshot() -> ConfigurationSnapshot {
    const KEY: u8 = 1;
    ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: PeerIdentity::new(HubId::new(), [KEY; ED25519_PUBLIC_KEY_BYTES]).unwrap(),
            revision: FIRST_CONFIGURATION_REVISION,
            last_change: Uuid::new_v4(),
            configuration: Configuration::default(),
        },
        applied: true,
        peer_socket: Some("/run/user/1000/yolo/hub/peer.sock".into()),
    }
}
pub fn configuration(snapshot: ConfigurationSnapshot) -> yolo_hub_rpc_protocol::Reply {
    yolo_hub_rpc_protocol::Reply::Configuration { snapshot }
}
pub fn manager() -> Manager {
    let snapshot = snapshot();
    let (mut manager, request) = Manager::open(snapshot.state.identity.hub());
    manager.complete(request.token, Ok(configuration(snapshot)));
    manager
}
pub fn edit(manager: &mut Manager) {
    manager.draft.as_mut().unwrap().trusted_origins.push(peer_identity());
}

pub fn key(manager: &mut Manager, code: crossterm::event::KeyCode) -> Option<Effect> {
    manager.handle(crossterm::event::Event::Key(
        crossterm::event::KeyEvent::new(code, crossterm::event::KeyModifiers::NONE),
    ))
}
pub fn activate(manager: &mut Manager, action: crate::model::Action) -> Option<Effect> {
    manager.selected = manager
        .items()
        .iter()
        .position(|item| item.action == action)
        .unwrap();
    key(manager, crossterm::event::KeyCode::Enter)
}
pub fn peer_identity() -> PeerIdentity {
    const KEY: u8 = 2;
    PeerIdentity::new(HubId::new(), [KEY; ED25519_PUBLIC_KEY_BYTES]).unwrap()
}
pub fn field(manager: &mut Manager, id: crate::fields::FieldId, value: &str) {
    let form = manager.form.as_mut().unwrap();
    form.focused = form.fields.iter().position(|field| field.id == id).unwrap();
    form.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('u'),
        crossterm::event::KeyModifiers::CONTROL,
    ));
    manager.handle(crossterm::event::Event::Paste(value.into()));
}
pub fn save(manager: &mut Manager) {
    let form = manager.form.as_mut().unwrap();
    form.focused = form.fields.len();
    key(manager, crossterm::event::KeyCode::Enter);
}
pub fn add_peer(manager: &mut Manager, identity: PeerIdentity) {
    use crate::{
        fields::FieldId,
        model::{Action, Page},
    };
    manager.page(Page::Peers);
    activate(manager, Action::AddPeer);
    field(manager, FieldId::Hub, &identity.hub().to_string());
    field(
        manager,
        FieldId::Key,
        &crate::wire_fields::key_text(identity),
    );
    // This helper adds a provider-only pin with no dial, so import defaults off.
    let form = manager.form.as_mut().unwrap();
    form.focused = form
        .fields
        .iter()
        .position(|field| field.id == FieldId::Import)
        .unwrap();
    key(manager, crossterm::event::KeyCode::Char(' '));
    save(manager);
    assert!(manager.form.is_none());
}
