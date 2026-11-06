use uuid::Uuid;
use yolo_broker_hub_protocol::*;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;

pub const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const HOST: &str = "hub-fixture";
pub const SOCKET: &str = "/run/user/1000/yolo/hub/peer.sock";
pub const PEER_KEY: [u8; 32] = [7; 32];
pub const GENERATION: u64 = 1;
pub const CHANNEL: u64 = 3;

pub fn profile() -> BrokerHubProfile {
    BrokerHubProfile {
        host_id: HOST.into(),
        profile_authority_sha256: HASH.into(),
        config_revision: HASH.into(),
    }
}
pub fn binding() -> BrokerHubConnectionBinding {
    BrokerHubConnectionBinding {
        profile: profile(),
        broker_instance_id: Uuid::new_v4(),
        connection_id: Uuid::new_v4(),
        connection_generation: GENERATION,
    }
}
pub fn request() -> BrokerHubOpenRequest {
    BrokerHubOpenRequest {
        binding: binding(),
        socket: SOCKET.into(),
        peer: PeerIdentity::new(HubId::new(), PEER_KEY).unwrap(),
    }
}
pub fn opened() -> BrokerHubOpened {
    BrokerHubOpened {
        request: request(),
        channel_id: CHANNEL,
    }
}
