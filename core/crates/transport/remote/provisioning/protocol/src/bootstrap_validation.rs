use crate::BootstrapError;
use yolo_hub_delegation::{MAX_DELEGATION_DEPTH, PeerIdentity};
use yolo_hub_routing::{Right, Rights};

pub(crate) fn required_peer<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PeerIdentity>, D::Error> {
    serde::Deserialize::deserialize(deserializer)
}

pub(crate) fn distinct(first: PeerIdentity, second: PeerIdentity) -> Result<(), BootstrapError> {
    if first.hub() == second.hub() || first.key() == second.key() {
        Err(BootstrapError::SameIdentity)
    } else {
        Ok(())
    }
}

pub(crate) fn authorization(rights: &Rights, depth: u8) -> Result<(), BootstrapError> {
    if depth > MAX_DELEGATION_DEPTH || !rights.allows(&[Right::Observe].into_iter().collect()) {
        Err(BootstrapError::Authorization)
    } else {
        Ok(())
    }
}
