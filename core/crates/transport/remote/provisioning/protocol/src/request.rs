use crate::{PROVISION_SCHEMA_VERSION, constants::INVALID};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use yolo_broker_hub_protocol::{BrokerHubConnectionBinding, BrokerHubProfile};
use yolo_distribution_bundle::{BundleSource, RELEASES_DIRECTORY};
use yolo_distribution_publication_protocol::validate_install_root;
use crate::RemoteBootstrapRequest;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProvisionFields", into = "ProvisionFields")]
pub struct ProvisionRequest(ProvisionFields);
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    pub profile: BrokerHubProfile,
    pub source: BundleSource,
    pub install_root: PathBuf,
    pub state: PathBuf,
    pub host_home: PathBuf,
    pub bootstrap: RemoteBootstrapRequest,
}
impl ProvisionRequest {
    pub fn new(fields: ProvisionFields) -> Result<Self> {
        fields.try_into()
    }
    pub fn fields(&self) -> &ProvisionFields {
        &self.0
    }
}
impl TryFrom<ProvisionFields> for ProvisionRequest {
    type Error = anyhow::Error;
    fn try_from(fields: ProvisionFields) -> Result<Self> {
        ensure!(
            fields.schema_version == PROVISION_SCHEMA_VERSION
                && !fields.request_id.is_nil()
                && fields.request_id == fields.bootstrap.fields().request_id
                && fields.source.descriptor() == &fields.bootstrap.fields().bundle,
            INVALID
        );
        fields.profile.validate()?;
        for path in [&fields.install_root, &fields.state, &fields.host_home] {
            validate_install_root(path)?;
        }
        let releases = fields.install_root.join(RELEASES_DIRECTORY);
        ensure!(
            !fields.state.starts_with(&releases)
                && !fields.host_home.starts_with(&releases)
                && fields.state != fields.host_home,
            INVALID
        );
        Ok(Self(fields))
    }
}
impl From<ProvisionRequest> for ProvisionFields {
    fn from(value: ProvisionRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerProvisionCall {
    pub binding: BrokerHubConnectionBinding,
    pub request: ProvisionRequest,
}
impl BrokerProvisionCall {
    pub fn validate(&self) -> Result<()> {
        self.binding.validate()?;
        ensure!(
            self.binding.profile == self.request.fields().profile,
            INVALID
        );
        Ok(())
    }
}
