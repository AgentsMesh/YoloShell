use yolo_hub_connector_protocol::ConnectorError;
use yolo_hub_identity::RuntimeTarget;

/// Trusted origin evidence, never deserialized from a Connector message.
#[derive(Clone, Copy, Debug)]
pub enum RuntimeAdmission {
    Available(RuntimeTarget),
    NativeOwned,
    Retired,
    Unavailable,
}
impl RuntimeAdmission {
    pub fn require(self, target: RuntimeTarget) -> Result<(), ConnectorError> {
        match self {
            Self::Available(observed) if observed == target => Ok(()),
            Self::Unavailable => Err(ConnectorError::Unavailable),
            _ => Err(ConnectorError::Conflict),
        }
    }
    pub(crate) fn supersedes(self, target: RuntimeTarget) -> bool {
        matches!(self, Self::NativeOwned | Self::Retired)
            || matches!(self, Self::Available(observed) if observed != target)
    }
}
