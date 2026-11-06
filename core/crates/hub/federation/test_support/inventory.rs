use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::HubSummary;

pub const LOCAL_ID: u128 = 1;
pub const REMOTE_ID: u128 = 2;
pub const LAST_ID: u128 = 3;
pub const ITEM_STEP: usize = 1;

pub fn hub(value: u128) -> HubId {
    HubId::from_uuid(uuid::Uuid::from_u128(value)).unwrap()
}

pub fn summary(value: u128, reachable: bool) -> HubSummary {
    HubSummary {
        hub_id: hub(value),
        reachable,
        label: None,
    }
}
