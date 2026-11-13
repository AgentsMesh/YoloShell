use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::HubId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutAddress {
    pub hub_id: HubId,
    #[serde(with = "crate::identity")]
    pub layout_id: Uuid,
}
