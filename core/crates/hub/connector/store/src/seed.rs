use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;
use yolo_crypto::Sha256Digest;
use yolo_hub_connector_protocol::{ConnectorCredential, ConnectorScope};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::{AgentAddress, ConnectorId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seed {
    pub connector_id: ConnectorId,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    pub agent: AgentAddress,
    pub scope: ConnectorScope,
    pub credential: ConnectorCredential,
}

impl Seed {
    pub fn validate(&self, identity: PeerIdentity) -> Result<()> {
        ensure!(
            !self.request_id.is_nil() && self.agent.hub_id == identity.hub(),
            INVALID_SEED
        );
        self.scope.validate_for(identity.hub())?;
        Ok(())
    }
    pub(crate) fn digest(&self) -> Result<Sha256Digest> {
        Sha256Digest::capture(&(SEED_DIGEST_DOMAIN, self))
    }
}

/// Validates a serialized state against its immutable enrollment seed.
/// The registry owns transition ordering and revision monotonicity.
pub trait StoredState: Clone + Eq + Serialize + DeserializeOwned {
    fn validate(&self, seed: &Seed) -> Result<()>;
}
