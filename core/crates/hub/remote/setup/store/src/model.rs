use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_remote_setup_protocol::RemoteSetupRequest;
use yolo_hub_ssh_profile_protocol::SshBrokerProfile;
use yolo_paths::identity::DurableObjectIdentity;
use yolo_remote_provisioning_protocol::ProvisionStatus;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "setup_phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum SetupPhase {
    Prepared {},
    Provisioning {
        broker_directory: DurableObjectIdentity,
    },
    Routing {
        broker_directory: DurableObjectIdentity,
        change: Option<ConfigurationChange>,
        provision: ProvisionStatus,
    },
    Complete {
        broker_directory: DurableObjectIdentity,
        change: Option<ConfigurationChange>,
        provision: ProvisionStatus,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupRecord {
    pub schema_version: u16,
    pub owner: PeerIdentity,
    pub request: RemoteSetupRequest,
    pub registration: SshBrokerProfile,
    pub baseline: ConfigurationSnapshot,
    pub phase: SetupPhase,
}
impl SetupRecord {
    pub fn new(
        owner: PeerIdentity,
        request: RemoteSetupRequest,
        registration: SshBrokerProfile,
        baseline: ConfigurationSnapshot,
    ) -> Result<Self> {
        let record = Self {
            schema_version: STORE_SCHEMA,
            owner,
            request,
            registration,
            baseline,
            phase: SetupPhase::Prepared {},
        };
        record.validate()?;
        Ok(record)
    }
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        self.registration.validate()?;
        self.baseline.validate()?;
        ensure!(
            self.schema_version == STORE_SCHEMA
                && self.baseline.applied
                && self.baseline.state.identity == self.owner
                && self.owner == self.request.request.fields().bootstrap.fields().consumer
                && self.request.expected_configuration_revision == self.baseline.state.revision
                && self.registration.profile == self.request.profile()?
                && self.registration.config_revision
                    == self.request.request.fields().profile.config_revision,
            CONFLICT
        );
        match &self.phase {
            SetupPhase::Prepared {} | SetupPhase::Provisioning { .. } => {}
            SetupPhase::Routing {
                change, provision, ..
            }
            | SetupPhase::Complete {
                change, provision, ..
            } => {
                provision.validate_for(&self.request.request)?;
                ensure!(
                    yolo_hub_remote_setup_protocol::plan_link(
                        &self.baseline,
                        &self.request,
                        provision
                    )? == *change,
                    CONFLICT
                );
                if let Some(change) = change {
                    change.validate(self.owner)?;
                    ensure!(
                        change.expected_revision == self.baseline.state.revision
                            && change.request_id == self.request.request.fields().request_id,
                        CONFLICT
                    );
                }
            }
        }
        Ok(())
    }
    pub fn broker_directory(&self) -> Option<DurableObjectIdentity> {
        match self.phase {
            SetupPhase::Prepared {} => None,
            SetupPhase::Provisioning { broker_directory }
            | SetupPhase::Routing {
                broker_directory, ..
            }
            | SetupPhase::Complete {
                broker_directory, ..
            } => Some(broker_directory),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoundRecord {
    pub parent: DurableObjectIdentity,
    pub directory: DurableObjectIdentity,
    pub record: SetupRecord,
}
