use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;

mod frames;

pub(crate) fn peer() -> PeerIdentity {
    PeerIdentity::new(HubId::new(), [0x21; 32]).unwrap()
}

pub(crate) fn request() -> yolo_hub_enrollment_wire::JoinRequest {
    yolo_hub_enrollment_wire::JoinRequest {
        schema_version: yolo_hub_enrollment_wire::JOIN_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        invite_id: Uuid::new_v4(),
        peer: peer(),
    }
}
