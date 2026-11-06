mod constants;
mod credentials;
mod history;
mod link;
use anyhow::{Result, ensure};
pub use constants::{
    MAX_PRIVATE_CREDENTIAL_BYTES, SETUP_CHECKPOINT_PHASE_TAG, SETUP_HISTORY_CHECKPOINT_FIELD,
    SETUP_HISTORY_OWNER_FIELD, SETUP_OPERATION_ID_FIELD,
};
pub use credentials::PrivateCredentials;
pub use history::{RemoteSetupHistory, SetupCheckpoint};
pub use link::{configuration_contains_link, plan_link};
use serde::{Deserialize, Serialize};
use yolo_hub_management_protocol::{ConfigurationSnapshot, FIRST_CONFIGURATION_REVISION};
use yolo_remote_provisioning_protocol::{ProvisionRequest, ProvisionStatus};
use yolo_ssh_profile_protocol::SshProfileRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupAction {
    Start,
    Inspect,
    Poll,
    Continue,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteSetupRequest {
    pub request: ProvisionRequest,
    pub expected_configuration_revision: u64,
}
impl RemoteSetupRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.expected_configuration_revision >= FIRST_CONFIGURATION_REVISION,
            constants::INVALID_REQUEST
        );
        Ok(())
    }
    pub fn profile(&self) -> Result<SshProfileRef> {
        let profile = &self.request.fields().profile;
        SshProfileRef::new(
            profile.host_id.clone(),
            profile.profile_authority_sha256.clone(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteSetupCall {
    pub action: SetupAction,
    pub request: RemoteSetupRequest,
    #[serde(default)]
    pub credentials: Option<PrivateCredentials>,
}
impl RemoteSetupCall {
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        ensure!(
            !matches!(self.action, SetupAction::Inspect | SetupAction::Poll)
                || self.credentials.is_none(),
            constants::INVALID_REQUEST
        );
        Ok(())
    }
    pub fn mutates(&self) -> bool {
        matches!(self.action, SetupAction::Start | SetupAction::Continue)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteSetupOutcome {
    Running {},
    Remote {
        provision: ProvisionStatus,
    },
    Linked {
        provision: ProvisionStatus,
        configuration: ConfigurationSnapshot,
    },
    Conflict {
        provision: ProvisionStatus,
    },
    Unknown {},
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteSetupStatus {
    pub request: RemoteSetupRequest,
    pub outcome: RemoteSetupOutcome,
}
impl RemoteSetupStatus {
    pub fn validate_for(&self, request: &RemoteSetupRequest) -> Result<()> {
        ensure!(&self.request == request, constants::INVALID_REQUEST);
        match &self.outcome {
            RemoteSetupOutcome::Remote { provision }
            | RemoteSetupOutcome::Conflict { provision } => {
                provision.validate_for(&request.request)?
            }
            RemoteSetupOutcome::Linked {
                provision,
                configuration,
            } => {
                provision.validate_for(&request.request)?;
                configuration.validate()?;
                ensure!(
                    configuration.applied
                        && configuration.state.identity
                            == request.request.fields().bootstrap.fields().consumer,
                    constants::INVALID_REQUEST
                );
                ensure!(
                    configuration_contains_link(configuration, provision)?,
                    constants::INVALID_REQUEST
                );
            }
            RemoteSetupOutcome::Unknown {} | RemoteSetupOutcome::Running {} => {}
        }
        Ok(())
    }
}
