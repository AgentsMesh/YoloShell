#[path = "../test_support/keys.rs"]
mod keys;
mod lifecycle;
mod profile;

use crate::{ENTRY_PROFILE_SCHEMA, EntryProfile};
use yolo_hub_client_transport::HubPin;
use yolo_hub_identity::HubId;

const HOST: &str = "example.test";
const PORT: u16 = 22;
const USERNAME: &str = "public-user";
const SOCKET: &str = "/private/yolo/public.sock";
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn profile() -> EntryProfile {
    let (public, key) = keys::keys();
    EntryProfile {
        schema_version: ENTRY_PROFILE_SCHEMA,
        host: HOST.into(),
        port: PORT,
        username: USERNAME.into(),
        outer_host_key: key.public_key().to_openssh().unwrap(),
        remote_socket: SOCKET.into(),
        hub: HubPin {
            hub_id: HubId::new(),
            public_key: public,
        },
        client_id: uuid::Uuid::new_v4(),
    }
}
mod constants;
