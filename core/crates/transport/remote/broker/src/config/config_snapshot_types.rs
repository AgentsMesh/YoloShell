use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct BrokerConfigGeneration {
    pub(super) hub_transport: bool,
    pub(super) config_path: PathBuf,
    pub(super) host_id: String,
    pub(super) profile_authority_sha256: String,
    pub(super) revision: String,
    pub(super) credential_binding_nonce: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::remote_broker) enum GenerationPublishStage {
    ExistingOpened,
    SourceVerified,
    DestinationRenamed,
    DestinationCommitted,
    ExistingDurabilityConfirmed,
}

impl BrokerConfigGeneration {
    pub fn hub_transport(&self) -> bool {
        self.hub_transport
    }
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn host_id(&self) -> &str {
        &self.host_id
    }

    pub fn profile_authority_sha256(&self) -> &str {
        &self.profile_authority_sha256
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn credential_binding_nonce(&self) -> Option<&str> {
        self.credential_binding_nonce.as_deref()
    }
}
