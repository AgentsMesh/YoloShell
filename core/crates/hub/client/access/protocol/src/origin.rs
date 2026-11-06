use crate::AccessError;
use serde::{Deserialize, Serialize};
use yolo_hub_identity::HubId;
use yolo_hub_routing::Rights;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "OriginWire")]
pub struct OriginAccess {
    pub origin: HubId,
    pub rights: Rights,
}

impl OriginAccess {
    pub fn validate(&self) -> Result<(), AccessError> {
        if self.rights.is_empty() {
            Err(AccessError::Grant)
        } else {
            Ok(())
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginWire {
    origin: HubId,
    rights: Rights,
}

impl TryFrom<OriginWire> for OriginAccess {
    type Error = AccessError;

    fn try_from(wire: OriginWire) -> Result<Self, Self::Error> {
        let origin = Self {
            origin: wire.origin,
            rights: wire.rights,
        };
        origin.validate()?;
        Ok(origin)
    }
}
