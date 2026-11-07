use crate::{BootstrapError, REMOTE_BOOTSTRAP_SCHEMA_VERSION, bootstrap_validation as validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_distribution_bundle::BundleDescriptor;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_routing::Rights;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "RemoteBootstrapRequestFields",
    into = "RemoteBootstrapRequestFields"
)]
pub struct RemoteBootstrapRequest(RemoteBootstrapRequestFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteBootstrapRequestFields {
    pub schema_version: u16,
    pub request_id: Uuid,
    pub bundle: BundleDescriptor,
    /// None is explicit first enrollment over the separately authenticated SSH login.
    #[serde(deserialize_with = "validation::required_peer")]
    pub expected_peer: Option<PeerIdentity>,
    pub consumer: PeerIdentity,
    pub rights: Rights,
    pub remaining_reexports: u8,
}

impl RemoteBootstrapRequest {
    pub fn new(fields: RemoteBootstrapRequestFields) -> Result<Self, BootstrapError> {
        if fields.schema_version != REMOTE_BOOTSTRAP_SCHEMA_VERSION {
            return Err(BootstrapError::Schema);
        }
        if fields.request_id.is_nil() {
            return Err(BootstrapError::RequestIdentity);
        }
        if let Some(peer) = fields.expected_peer {
            validation::distinct(peer, fields.consumer)?;
        }
        validation::authorization(&fields.rights, fields.remaining_reexports)?;
        Ok(Self(fields))
    }
    pub fn fields(&self) -> &RemoteBootstrapRequestFields {
        &self.0
    }
}
impl TryFrom<RemoteBootstrapRequestFields> for RemoteBootstrapRequest {
    type Error = BootstrapError;
    fn try_from(fields: RemoteBootstrapRequestFields) -> Result<Self, Self::Error> {
        Self::new(fields)
    }
}
impl From<RemoteBootstrapRequest> for RemoteBootstrapRequestFields {
    fn from(value: RemoteBootstrapRequest) -> Self {
        value.0
    }
}
