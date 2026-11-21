use crate::{BootstrapError, RemoteBootstrapRequest, bootstrap_constants::*, bootstrap_validation as validation};

const MAX_SOCKET_PATH_BYTES: usize = 100;
use serde::{Deserialize, Serialize};
use std::path::{Component, PathBuf};
use uuid::Uuid;
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_routing::Rights;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "RemoteBootstrapReceiptFields",
    into = "RemoteBootstrapReceiptFields"
)]
pub struct RemoteBootstrapReceipt(RemoteBootstrapReceiptFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteBootstrapReceiptFields {
    pub schema_version: u16,
    pub request_id: Uuid,
    pub peer: PeerIdentity,
    pub peer_socket: PathBuf,
    pub configuration_revision: u64,
    pub bundle_id: String,
    pub build: ServiceBuild,
    pub consumer: PeerIdentity,
    pub rights: Rights,
    pub remaining_reexports: u8,
}

fn valid_socket(path: &std::path::Path) -> bool {
    let encoded = path.as_os_str().as_encoded_bytes();
    path.is_absolute()
        && path.to_str().is_some_and(|raw| {
            !raw.split('/').any(|part| part == "." || part == "..")
        })
        && !encoded.is_empty()
        && encoded.len() <= MAX_SOCKET_PATH_BYTES
        && !encoded.iter().any(|byte| byte.is_ascii_control())
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

impl RemoteBootstrapReceipt {
    pub fn new(fields: RemoteBootstrapReceiptFields) -> Result<Self, BootstrapError> {
        if fields.schema_version != REMOTE_BOOTSTRAP_SCHEMA_VERSION {
            return Err(BootstrapError::Schema);
        }
        if fields.request_id.is_nil() {
            return Err(BootstrapError::RequestIdentity);
        }
        validation::distinct(fields.peer, fields.consumer)?;
        validation::authorization(&fields.rights, fields.remaining_reexports)?;
        if !valid_socket(&fields.peer_socket) {
            return Err(BootstrapError::Endpoint);
        }
        if fields.configuration_revision < FIRST_CONFIGURATION_REVISION {
            return Err(BootstrapError::Revision);
        }
        if fields.bundle_id.len() != yolo_distribution_bundle::SHA256_HEX_BYTES
            || !fields
                .bundle_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(BootstrapError::Correlation);
        }
        Ok(Self(fields))
    }

    pub fn fields(&self) -> &RemoteBootstrapReceiptFields {
        &self.0
    }

    /// This correlates observations; peer SSH authentication still grants admission.
    pub fn validate_for(&self, request: &RemoteBootstrapRequest) -> Result<(), BootstrapError> {
        let expected = request.fields();
        let actual = self.fields();
        if actual.request_id != expected.request_id
            || actual.consumer != expected.consumer
            || actual.rights != expected.rights
            || actual.remaining_reexports != expected.remaining_reexports
            || actual.bundle_id != expected.bundle.build_id()
        {
            return Err(BootstrapError::Correlation);
        }
        if expected
            .expected_peer
            .is_some_and(|peer| peer != actual.peer)
        {
            return Err(BootstrapError::PeerMismatch);
        }
        use yolo_distribution_bundle::ArtifactRole;
        let artifact = expected.bundle.artifact(ArtifactRole::Runtime);
        for image in [
            &actual.build.host,
            &actual.build.hub,
            &actual.build.file_helper,
        ] {
            if image.sha256() != artifact.sha256() || image.size_bytes() != artifact.size_bytes() {
                return Err(BootstrapError::UpgradeRequired);
            }
        }
        Ok(())
    }
}
impl TryFrom<RemoteBootstrapReceiptFields> for RemoteBootstrapReceipt {
    type Error = BootstrapError;
    fn try_from(fields: RemoteBootstrapReceiptFields) -> Result<Self, Self::Error> {
        Self::new(fields)
    }
}
impl From<RemoteBootstrapReceipt> for RemoteBootstrapReceiptFields {
    fn from(value: RemoteBootstrapReceipt) -> Self {
        value.0
    }
}
